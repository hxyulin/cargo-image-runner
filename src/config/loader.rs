use super::Config;
use crate::core::error::{Error, Result};
use cargo_metadata::MetadataCommand;
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
    /// 2. Cargo.toml metadata
    /// 3. Standalone TOML file
    pub fn load(self) -> Result<(Config, PathBuf)> {
        let mut config = Config::default();
        let workspace_root;

        // Load from Cargo metadata if enabled
        if self.use_cargo_metadata {
            let (root, cargo_config) = self.load_cargo_metadata()?;
            workspace_root = root;
            config = Self::merge_configs(config, cargo_config);
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

        Ok((config, workspace_root))
    }

    /// Load configuration from Cargo.toml metadata.
    ///
    /// Priority: package metadata > workspace metadata > defaults.
    fn load_cargo_metadata(&self) -> Result<(PathBuf, Config)> {
        let manifest_path = std::env::var("CARGO_MANIFEST_PATH").ok();

        let mut cmd = MetadataCommand::new();
        if let Some(manifest_path) = manifest_path {
            cmd.manifest_path(manifest_path);
        }

        let metadata = cmd.exec()?;
        let workspace_root = metadata.workspace_root.clone().into_std_path_buf();

        // Parse workspace metadata: [workspace.metadata.image-runner]
        let workspace_config = if let Some(ws_value) = metadata.workspace_metadata.get("image-runner") {
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

        Ok((workspace_root, config))
    }

    /// Load configuration from a standalone TOML file.
    fn load_toml_file(&self, path: &Path) -> Result<Config> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::config(format!("failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| Error::config(format!("failed to parse TOML config: {}", e)))
    }

    /// Merge two configurations, with `override_config` taking precedence.
    fn merge_configs(mut base: Config, override_cfg: Config) -> Config {
        // For now, we do a simple override strategy
        // In Phase 5, we'll implement more sophisticated merging
        // that handles individual fields properly

        // Simple merge: non-default values from override take precedence
        // This is a placeholder - full implementation in Phase 5
        base.boot = override_cfg.boot;
        base.bootloader = override_cfg.bootloader;
        base.image = override_cfg.image;
        base.runner = override_cfg.runner;
        base.test = override_cfg.test;
        base.run = override_cfg.run;

        // Merge variables
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
