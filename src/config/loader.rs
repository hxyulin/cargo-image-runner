use super::Config;
use crate::core::error::{Error, Result};
use cargo_metadata::MetadataCommand;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Configuration loader that supports multiple sources.
pub struct ConfigLoader {
    /// Path to workspace root.
    workspace_root: Option<PathBuf>,
    /// Path to standalone config file.
    config_file: Option<PathBuf>,
    /// Whether to load from Cargo.toml metadata.
    use_cargo_metadata: bool,
}

impl ConfigLoader {
    /// Create a new configuration loader.
    pub fn new() -> Self {
        Self {
            workspace_root: None,
            config_file: None,
            use_cargo_metadata: true,
        }
    }

    /// Set the workspace root directory.
    pub fn workspace_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    /// Set a standalone configuration file path.
    pub fn config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_file = Some(path.into());
        self
    }

    /// Disable loading from Cargo.toml metadata.
    pub fn no_cargo_metadata(mut self) -> Self {
        self.use_cargo_metadata = false;
        self
    }

    /// Load configuration from all enabled sources.
    ///
    /// Priority (later sources override earlier):
    /// 1. Default values
    /// 2. Cargo.toml metadata (workspace then package)
    /// 3. Standalone TOML file
    /// 4. Profile overlay (`CARGO_IMAGE_RUNNER_PROFILE`)
    /// 5. Individual env var overrides (`CARGO_IMAGE_RUNNER_*`)
    pub fn load(self) -> Result<(Config, PathBuf)> {
        let mut config = Config::default();
        let workspace_root;
        let mut profiles: HashMap<String, serde_json::Value> = HashMap::new();

        // Load from Cargo metadata if enabled
        if self.use_cargo_metadata {
            let (root, cargo_config, cargo_profiles) = self.load_cargo_metadata()?;
            workspace_root = root;
            config = Self::merge_configs(config, cargo_config);
            profiles = cargo_profiles;
        } else {
            workspace_root = self
                .workspace_root
                .clone()
                .ok_or_else(|| Error::config("workspace root not specified"))?;
        }

        // Load from standalone file if specified
        if let Some(ref config_path) = self.config_file {
            let file_config = self.load_toml_file(config_path)?;
            config = Self::merge_configs(config, file_config);
        }

        // Apply profile overlay if CARGO_IMAGE_RUNNER_PROFILE is set
        if let Some(profile_name) = super::env::get_profile_name() {
            let profile_value = profiles.get(&profile_name).ok_or_else(|| {
                let available: Vec<&String> = profiles.keys().collect();
                if available.is_empty() {
                    Error::config(format!(
                        "profile '{}' not found (no profiles defined)",
                        profile_name,
                    ))
                } else {
                    Error::config(format!(
                        "profile '{}' not found. Available profiles: {}",
                        profile_name,
                        available.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                    ))
                }
            })?;

            let mut base_value = serde_json::to_value(&config)
                .map_err(|e| Error::config(format!("failed to serialize config: {}", e)))?;
            deep_merge(&mut base_value, profile_value);
            config = serde_json::from_value(base_value)
                .map_err(|e| Error::config(format!("failed to apply profile '{}': {}", profile_name, e)))?;
        }

        // Apply individual env var overrides (highest priority)
        super::env::apply_env_overrides(&mut config);

        Ok((config, workspace_root))
    }

    /// Load configuration from Cargo.toml metadata.
    ///
    /// Returns `(workspace_root, config, profiles)`.
    /// Priority: package metadata > workspace metadata > defaults.
    /// Profiles are collected from both workspace and package metadata
    /// (package profiles override workspace profiles with the same name).
    fn load_cargo_metadata(
        &self,
    ) -> Result<(PathBuf, Config, HashMap<String, serde_json::Value>)> {
        let manifest_path = std::env::var("CARGO_MANIFEST_PATH").ok();

        let mut cmd = MetadataCommand::new();
        if let Some(manifest_path) = manifest_path {
            cmd.manifest_path(manifest_path);
        }

        let metadata = cmd.exec()?;
        let workspace_root = metadata.workspace_root.clone().into_std_path_buf();

        let mut profiles: HashMap<String, serde_json::Value> = HashMap::new();

        // Parse workspace metadata: [workspace.metadata.image-runner]
        let workspace_config = if let Some(ws_value) = metadata.workspace_metadata.get("image-runner") {
            // Extract profiles before deserializing Config
            extract_profiles(ws_value, &mut profiles);

            Some(
                serde_json::from_value::<Config>(ws_value.clone())
                    .map_err(|e| Error::config(format!("invalid workspace metadata: {}", e)))?,
            )
        } else {
            None
        };

        // Try to find the package metadata
        let pkg_name = std::env::var("CARGO_PKG_NAME").ok();
        let package = if let Some(ref pkg_name) = pkg_name {
            metadata
                .packages
                .iter()
                .find(|p| &p.name == pkg_name)
                .or_else(|| metadata.root_package())
        } else {
            metadata.root_package()
        };

        // Parse package metadata: [package.metadata.image-runner]
        let package_config = if let Some(package) = package {
            if let Some(metadata_value) = package.metadata.get("image-runner") {
                // Package profiles override workspace profiles
                extract_profiles(metadata_value, &mut profiles);

                Some(
                    serde_json::from_value::<Config>(metadata_value.clone())
                        .map_err(|e| Error::config(format!("invalid Cargo.toml metadata: {}", e)))?,
                )
            } else {
                None
            }
        } else {
            None
        };

        // Merge: defaults <- workspace <- package
        let mut config = Config::default();
        if let Some(ws_config) = workspace_config {
            config = Self::merge_configs(config, ws_config);
        }
        if let Some(pkg_config) = package_config {
            config = Self::merge_configs(config, pkg_config);
        }

        Ok((workspace_root, config, profiles))
    }

    /// Load configuration from a standalone TOML file.
    fn load_toml_file(&self, path: &Path) -> Result<Config> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::config(format!("failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| Error::config(format!("failed to parse TOML config: {}", e)))
    }

    /// Merge two configurations, with `override_config` taking precedence.
    pub(crate) fn merge_configs(mut base: Config, override_cfg: Config) -> Config {
        base.boot = override_cfg.boot;
        base.bootloader = override_cfg.bootloader;
        base.image = override_cfg.image;
        base.runner = override_cfg.runner;
        base.test = override_cfg.test;
        base.run = override_cfg.run;
        base.verbose = override_cfg.verbose;

        // Merge variables (override wins per-key, base keys preserved)
        for (k, v) in override_cfg.variables {
            base.variables.insert(k, v);
        }

        base
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract profile definitions from a metadata JSON value.
///
/// Profiles live at `value["profiles"]` as `{ name: { ...config fields... } }`.
/// Package-level profiles override workspace-level profiles with the same name.
fn extract_profiles(
    value: &serde_json::Value,
    profiles: &mut HashMap<String, serde_json::Value>,
) {
    if let Some(serde_json::Value::Object(map)) = value.get("profiles") {
        for (name, profile_value) in map {
            profiles.insert(name.clone(), profile_value.clone());
        }
    }
}

/// Recursively deep-merge `overlay` into `base`.
///
/// - Objects: keys are merged recursively (overlay keys win for conflicts).
/// - Scalars and arrays: overlay replaces base entirely.
pub(crate) fn deep_merge(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let entry = base_map
                    .entry(key.clone())
                    .or_insert(serde_json::Value::Null);
                deep_merge(entry, overlay_val);
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BootType, BootloaderKind, ImageFormat};

    #[test]
    fn test_load_standalone_toml_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("image-runner.toml");
        std::fs::write(
            &config_path,
            r#"
[boot]
type = "hybrid"

[bootloader]
kind = "limine"
config-file = "limine.conf"

[image]
format = "iso"

[variables]
TIMEOUT = "5"
"#,
        )
        .unwrap();

        let loader = ConfigLoader::new()
            .no_cargo_metadata()
            .workspace_root(dir.path())
            .config_file(&config_path);
        let (config, root) = loader.load().unwrap();

        assert_eq!(config.boot.boot_type, BootType::Hybrid);
        assert_eq!(config.bootloader.kind, BootloaderKind::Limine);
        assert_eq!(config.image.format, ImageFormat::Iso);
        assert_eq!(config.variables.get("TIMEOUT").unwrap(), "5");
        assert_eq!(root, dir.path());
    }

    #[test]
    fn test_merge_configs_override_behavior() {
        let base = Config::default();
        let mut override_cfg = Config::default();
        override_cfg.boot.boot_type = BootType::Hybrid;
        override_cfg.bootloader.kind = BootloaderKind::Limine;
        override_cfg.image.format = ImageFormat::Iso;

        let merged = ConfigLoader::merge_configs(base, override_cfg);
        assert_eq!(merged.boot.boot_type, BootType::Hybrid);
        assert_eq!(merged.bootloader.kind, BootloaderKind::Limine);
        assert_eq!(merged.image.format, ImageFormat::Iso);
    }

    #[test]
    fn test_merge_configs_variable_merging() {
        let mut base = Config::default();
        base.variables
            .insert("A".to_string(), "base_a".to_string());
        base.variables
            .insert("B".to_string(), "base_b".to_string());

        let mut override_cfg = Config::default();
        override_cfg
            .variables
            .insert("B".to_string(), "override_b".to_string());
        override_cfg
            .variables
            .insert("C".to_string(), "override_c".to_string());

        let merged = ConfigLoader::merge_configs(base, override_cfg);
        assert_eq!(merged.variables.get("A").unwrap(), "base_a");
        assert_eq!(merged.variables.get("B").unwrap(), "override_b");
        assert_eq!(merged.variables.get("C").unwrap(), "override_c");
    }

    #[test]
    fn test_missing_config_file_error() {
        let loader = ConfigLoader::new()
            .no_cargo_metadata()
            .workspace_root("/tmp")
            .config_file("/nonexistent/config.toml");
        let result = loader.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bad.toml");
        std::fs::write(&config_path, "this is not valid { toml [[[").unwrap();

        let loader = ConfigLoader::new()
            .no_cargo_metadata()
            .workspace_root(dir.path())
            .config_file(&config_path);
        let result = loader.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_no_cargo_metadata_requires_workspace_root() {
        let loader = ConfigLoader::new().no_cargo_metadata();
        let result = loader.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_deep_merge_objects() {
        let mut base = serde_json::json!({
            "boot": { "type": "uefi" },
            "runner": { "qemu": { "memory": 1024, "cores": 1 } }
        });
        let overlay = serde_json::json!({
            "runner": { "qemu": { "memory": 4096 } }
        });
        deep_merge(&mut base, &overlay);
        // memory overridden, cores preserved, boot preserved
        assert_eq!(base["runner"]["qemu"]["memory"], 4096);
        assert_eq!(base["runner"]["qemu"]["cores"], 1);
        assert_eq!(base["boot"]["type"], "uefi");
    }

    #[test]
    fn test_deep_merge_array_replaces() {
        let mut base = serde_json::json!({
            "runner": { "qemu": { "extra_args": ["-serial", "stdio"] } }
        });
        let overlay = serde_json::json!({
            "runner": { "qemu": { "extra_args": ["-s", "-S"] } }
        });
        deep_merge(&mut base, &overlay);
        assert_eq!(
            base["runner"]["qemu"]["extra_args"],
            serde_json::json!(["-s", "-S"])
        );
    }

    #[test]
    fn test_deep_merge_scalar_replaces() {
        let mut base = serde_json::json!({ "verbose": false });
        let overlay = serde_json::json!({ "verbose": true });
        deep_merge(&mut base, &overlay);
        assert_eq!(base["verbose"], true);
    }

    #[test]
    fn test_extract_profiles_from_json() {
        let value = serde_json::json!({
            "boot": { "type": "uefi" },
            "profiles": {
                "debug": {
                    "verbose": true,
                    "runner": { "qemu": { "memory": 4096 } }
                },
                "ci": {
                    "runner": { "qemu": { "kvm": false } }
                }
            }
        });
        let mut profiles = HashMap::new();
        extract_profiles(&value, &mut profiles);
        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains_key("debug"));
        assert!(profiles.contains_key("ci"));
        assert_eq!(profiles["debug"]["verbose"], true);
    }

    #[test]
    fn test_extract_profiles_none() {
        let value = serde_json::json!({ "boot": { "type": "uefi" } });
        let mut profiles = HashMap::new();
        extract_profiles(&value, &mut profiles);
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_profile_application_via_deep_merge() {
        // Simulate what load() does: serialize config, merge profile, deserialize
        let config = Config::default();
        let mut base_value = serde_json::to_value(&config).unwrap();

        let profile = serde_json::json!({
            "verbose": true,
            "runner": { "qemu": { "memory": 4096 } }
        });
        deep_merge(&mut base_value, &profile);

        let result: Config = serde_json::from_value(base_value).unwrap();
        assert!(result.verbose);
        assert_eq!(result.runner.qemu.memory, 4096);
        // Other defaults preserved
        assert_eq!(result.runner.qemu.cores, 1);
        assert_eq!(result.boot.boot_type, BootType::Uefi);
    }
}
