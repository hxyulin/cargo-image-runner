//! Environment variable processing for runtime configuration overrides.
//!
//! Env var prefix: `CARGO_IMAGE_RUNNER_`
//!
//! - `CARGO_IMAGE_RUNNER_PROFILE` — select a configuration profile
//! - `CARGO_IMAGE_RUNNER_QEMU_ARGS` — extra QEMU arguments (whitespace-split)
//! - `CARGO_IMAGE_RUNNER_VAR_<NAME>` — set template variable `<NAME>`
//! - `CARGO_IMAGE_RUNNER_QEMU_BINARY` — override QEMU binary path
//! - `CARGO_IMAGE_RUNNER_QEMU_MEMORY` — override memory (MB)
//! - `CARGO_IMAGE_RUNNER_QEMU_CORES` — override CPU cores
//! - `CARGO_IMAGE_RUNNER_QEMU_MACHINE` — override machine type
//! - `CARGO_IMAGE_RUNNER_BOOT_TYPE` — override boot type (bios/uefi/hybrid)
//! - `CARGO_IMAGE_RUNNER_VERBOSE` — enable verbose output (1/true/yes)
//! - `CARGO_IMAGE_RUNNER_KVM` — enable/disable KVM (1/true/yes or 0/false/no)

use super::Config;
use std::collections::HashMap;

const PREFIX: &str = "CARGO_IMAGE_RUNNER_";
const VAR_PREFIX: &str = "CARGO_IMAGE_RUNNER_VAR_";

/// Read the active profile name from `CARGO_IMAGE_RUNNER_PROFILE`.
pub fn get_profile_name() -> Option<String> {
    std::env::var("CARGO_IMAGE_RUNNER_PROFILE").ok().filter(|s| !s.is_empty())
}

/// Collect template variables from `CARGO_IMAGE_RUNNER_VAR_*` env vars.
///
/// The `VAR_` prefix is stripped, so `CARGO_IMAGE_RUNNER_VAR_TIMEOUT=10`
/// produces `("TIMEOUT", "10")`.
pub fn collect_env_variables() -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for (key, value) in std::env::vars() {
        if let Some(name) = key.strip_prefix(VAR_PREFIX) {
            if !name.is_empty() {
                vars.insert(name.to_string(), value);
            }
        }
    }
    vars
}

/// Parse `CARGO_IMAGE_RUNNER_QEMU_ARGS` into a list of arguments.
///
/// Arguments are split on whitespace. Returns an empty vec if unset.
pub fn get_extra_qemu_args() -> Vec<String> {
    match std::env::var(format!("{PREFIX}QEMU_ARGS")) {
        Ok(val) if !val.is_empty() => val.split_whitespace().map(String::from).collect(),
        _ => Vec::new(),
    }
}

/// Apply individual env var overrides to a config.
///
/// Each override is applied only if the env var is set and parses correctly.
/// Invalid values are silently ignored.
pub fn apply_env_overrides(config: &mut Config) {
    if let Some(val) = env_str("QEMU_BINARY") {
        config.runner.qemu.binary = val;
    }

    if let Some(val) = env_parse::<u32>("QEMU_MEMORY") {
        config.runner.qemu.memory = val;
    }

    if let Some(val) = env_parse::<u32>("QEMU_CORES") {
        config.runner.qemu.cores = val;
    }

    if let Some(val) = env_str("QEMU_MACHINE") {
        config.runner.qemu.machine = val;
    }

    if let Some(val) = env_str("BOOT_TYPE") {
        match val.to_lowercase().as_str() {
            "bios" => config.boot.boot_type = super::BootType::Bios,
            "uefi" => config.boot.boot_type = super::BootType::Uefi,
            "hybrid" => config.boot.boot_type = super::BootType::Hybrid,
            _ => {} // invalid value, ignore
        }
    }

    if let Some(val) = env_bool("VERBOSE") {
        config.verbose = val;
    }

    if let Some(val) = env_bool("KVM") {
        config.runner.qemu.kvm = val;
    }
}

/// Summarize which env var overrides are currently active.
///
/// Returns a list of `(env_var_name, value)` pairs for display in `check`.
pub fn detect_active_overrides() -> Vec<(String, String)> {
    let keys = [
        "QEMU_BINARY",
        "QEMU_MEMORY",
        "QEMU_CORES",
        "QEMU_MACHINE",
        "BOOT_TYPE",
        "VERBOSE",
        "KVM",
        "QEMU_ARGS",
    ];

    let mut active = Vec::new();
    for key in keys {
        let full = format!("{PREFIX}{key}");
        if let Ok(val) = std::env::var(&full) {
            if !val.is_empty() {
                active.push((full, val));
            }
        }
    }
    active
}

// --- helpers ---

fn env_str(suffix: &str) -> Option<String> {
    std::env::var(format!("{PREFIX}{suffix}"))
        .ok()
        .filter(|s| !s.is_empty())
}

fn env_parse<T: std::str::FromStr>(suffix: &str) -> Option<T> {
    env_str(suffix).and_then(|s| s.parse().ok())
}

fn env_bool(suffix: &str) -> Option<bool> {
    env_str(suffix).map(|s| matches!(s.to_lowercase().as_str(), "1" | "true" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Env vars are process-global, so serialize tests that mutate them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Helper: run a closure with specific env vars set, then restore.
    fn with_env_vars<F: FnOnce()>(vars: &[(&str, &str)], f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut old: Vec<(&str, Option<String>)> = Vec::new();
        for &(k, v) in vars {
            old.push((k, std::env::var(k).ok()));
            // SAFETY: tests are serialized via ENV_LOCK
            unsafe { std::env::set_var(k, v) };
        }
        f();
        for (k, prev) in old {
            // SAFETY: tests are serialized via ENV_LOCK
            match prev {
                Some(v) => unsafe { std::env::set_var(k, v) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }

    /// Helper: run with specific env vars removed.
    fn without_env_vars<F: FnOnce()>(vars: &[&str], f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let mut old: Vec<(&str, Option<String>)> = Vec::new();
        for &k in vars {
            old.push((k, std::env::var(k).ok()));
            // SAFETY: tests are serialized via ENV_LOCK
            unsafe { std::env::remove_var(k) };
        }
        f();
        for (k, prev) in old {
            // SAFETY: tests are serialized via ENV_LOCK
            match prev {
                Some(v) => unsafe { std::env::set_var(k, v) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }

    #[test]
    fn test_get_profile_name_set() {
        with_env_vars(&[("CARGO_IMAGE_RUNNER_PROFILE", "debug")], || {
            assert_eq!(get_profile_name(), Some("debug".to_string()));
        });
    }

    #[test]
    fn test_get_profile_name_unset() {
        without_env_vars(&["CARGO_IMAGE_RUNNER_PROFILE"], || {
            assert_eq!(get_profile_name(), None);
        });
    }

    #[test]
    fn test_get_profile_name_empty() {
        with_env_vars(&[("CARGO_IMAGE_RUNNER_PROFILE", "")], || {
            assert_eq!(get_profile_name(), None);
        });
    }

    #[test]
    fn test_collect_env_variables_multiple() {
        with_env_vars(
            &[
                ("CARGO_IMAGE_RUNNER_VAR_TIMEOUT", "10"),
                ("CARGO_IMAGE_RUNNER_VAR_DEBUG", "1"),
            ],
            || {
                let vars = collect_env_variables();
                assert_eq!(vars.get("TIMEOUT").unwrap(), "10");
                assert_eq!(vars.get("DEBUG").unwrap(), "1");
            },
        );
    }

    #[test]
    fn test_collect_env_variables_none() {
        without_env_vars(
            &[
                "CARGO_IMAGE_RUNNER_VAR_TIMEOUT",
                "CARGO_IMAGE_RUNNER_VAR_DEBUG",
            ],
            || {
                let vars = collect_env_variables();
                assert!(!vars.contains_key("TIMEOUT"));
                assert!(!vars.contains_key("DEBUG"));
            },
        );
    }

    #[test]
    fn test_get_extra_qemu_args_set() {
        with_env_vars(
            &[("CARGO_IMAGE_RUNNER_QEMU_ARGS", "-s -S -device virtio-net")],
            || {
                let args = get_extra_qemu_args();
                assert_eq!(args, vec!["-s", "-S", "-device", "virtio-net"]);
            },
        );
    }

    #[test]
    fn test_get_extra_qemu_args_empty() {
        with_env_vars(&[("CARGO_IMAGE_RUNNER_QEMU_ARGS", "")], || {
            assert!(get_extra_qemu_args().is_empty());
        });
    }

    #[test]
    fn test_get_extra_qemu_args_unset() {
        without_env_vars(&["CARGO_IMAGE_RUNNER_QEMU_ARGS"], || {
            assert!(get_extra_qemu_args().is_empty());
        });
    }

    #[test]
    fn test_apply_env_overrides_qemu_fields() {
        with_env_vars(
            &[
                ("CARGO_IMAGE_RUNNER_QEMU_BINARY", "my-qemu"),
                ("CARGO_IMAGE_RUNNER_QEMU_MEMORY", "4096"),
                ("CARGO_IMAGE_RUNNER_QEMU_CORES", "4"),
                ("CARGO_IMAGE_RUNNER_QEMU_MACHINE", "virt"),
            ],
            || {
                let mut config = Config::default();
                apply_env_overrides(&mut config);
                assert_eq!(config.runner.qemu.binary, "my-qemu");
                assert_eq!(config.runner.qemu.memory, 4096);
                assert_eq!(config.runner.qemu.cores, 4);
                assert_eq!(config.runner.qemu.machine, "virt");
            },
        );
    }

    #[test]
    fn test_apply_env_overrides_boot_type() {
        with_env_vars(&[("CARGO_IMAGE_RUNNER_BOOT_TYPE", "bios")], || {
            let mut config = Config::default();
            apply_env_overrides(&mut config);
            assert_eq!(config.boot.boot_type, super::super::BootType::Bios);
        });
    }

    #[test]
    fn test_apply_env_overrides_verbose_and_kvm() {
        with_env_vars(
            &[
                ("CARGO_IMAGE_RUNNER_VERBOSE", "yes"),
                ("CARGO_IMAGE_RUNNER_KVM", "0"),
            ],
            || {
                let mut config = Config::default();
                apply_env_overrides(&mut config);
                assert!(config.verbose);
                assert!(!config.runner.qemu.kvm);
            },
        );
    }

    #[test]
    fn test_apply_env_overrides_invalid_memory_ignored() {
        with_env_vars(&[("CARGO_IMAGE_RUNNER_QEMU_MEMORY", "notanumber")], || {
            let mut config = Config::default();
            let original_memory = config.runner.qemu.memory;
            apply_env_overrides(&mut config);
            assert_eq!(config.runner.qemu.memory, original_memory);
        });
    }

    #[test]
    fn test_apply_env_overrides_invalid_boot_type_ignored() {
        with_env_vars(&[("CARGO_IMAGE_RUNNER_BOOT_TYPE", "invalid")], || {
            let mut config = Config::default();
            let original = config.boot.boot_type;
            apply_env_overrides(&mut config);
            assert_eq!(config.boot.boot_type, original);
        });
    }
}
