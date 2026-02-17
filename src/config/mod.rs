//! Configuration types and loading from `[package.metadata.image-runner]` in Cargo.toml.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod env;
mod loader;
pub use loader::ConfigLoader;

/// Complete configuration for image runner.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Boot type configuration.
    #[serde(default)]
    pub boot: BootConfig,

    /// Bootloader configuration.
    #[serde(default)]
    pub bootloader: BootloaderConfig,

    /// Image format configuration.
    #[serde(default)]
    pub image: ImageConfig,

    /// Runner configuration.
    #[serde(default)]
    pub runner: RunnerConfig,

    /// Test-specific configuration.
    #[serde(default)]
    pub test: TestConfig,

    /// Run-specific configuration (non-test).
    #[serde(default)]
    pub run: RunConfig,

    /// Template variables for substitution.
    #[serde(default)]
    pub variables: HashMap<String, String>,

    /// Enable verbose output (show build progress messages).
    #[serde(default)]
    pub verbose: bool,
}

/// Boot type configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    /// Boot type: BIOS, UEFI, or Hybrid.
    #[serde(rename = "type")]
    pub boot_type: BootType,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            boot_type: BootType::Uefi,
        }
    }
}

/// Boot type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootType {
    /// BIOS boot only.
    Bios,
    /// UEFI boot only.
    Uefi,
    /// Support both BIOS and UEFI.
    Hybrid,
}

impl BootType {
    /// Check if BIOS boot is required.
    pub fn needs_bios(self) -> bool {
        matches!(self, BootType::Bios | BootType::Hybrid)
    }

    /// Check if UEFI boot is required.
    pub fn needs_uefi(self) -> bool {
        matches!(self, BootType::Uefi | BootType::Hybrid)
    }
}

/// Bootloader configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootloaderConfig {
    /// Bootloader type.
    pub kind: BootloaderKind,

    /// Path to bootloader configuration file.
    #[serde(rename = "config-file")]
    pub config_file: Option<PathBuf>,

    /// Additional files to include.
    #[serde(default, rename = "extra-files")]
    pub extra_files: Vec<PathBuf>,

    /// Limine-specific configuration.
    #[serde(default)]
    pub limine: LimineConfig,

    /// GRUB-specific configuration.
    #[serde(default)]
    pub grub: GrubConfig,
}

/// Bootloader type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BootloaderKind {
    /// Limine bootloader.
    Limine,
    /// GRUB bootloader.
    Grub,
    /// No bootloader (direct boot).
    #[default]
    None,
}

/// Limine bootloader configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimineConfig {
    /// Limine version to use.
    pub version: String,
}

impl Default for LimineConfig {
    fn default() -> Self {
        Self {
            version: "v8.x-binary".to_string(),
        }
    }
}

/// GRUB bootloader configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrubConfig {
    /// GRUB modules to include.
    #[serde(default)]
    pub modules: Vec<String>,
}

/// Image format configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Image format type.
    pub format: ImageFormat,

    /// Output path for the image.
    pub output: Option<PathBuf>,

    /// Volume label (for ISO/FAT).
    #[serde(default = "default_volume_label")]
    pub volume_label: String,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            format: ImageFormat::Directory,
            output: None,
            volume_label: default_volume_label(),
        }
    }
}

fn default_volume_label() -> String {
    "BOOT".to_string()
}

/// Image format enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// ISO 9660 image.
    Iso,
    /// FAT filesystem image.
    Fat,
    /// Directory (for QEMU fat:rw:).
    #[default]
    Directory,
}

/// Runner configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunnerConfig {
    /// Runner type.
    pub kind: RunnerKind,

    /// QEMU-specific configuration.
    #[serde(default)]
    pub qemu: QemuConfig,
}

/// Runner type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RunnerKind {
    /// QEMU emulator.
    #[default]
    Qemu,
}

/// QEMU runner configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QemuConfig {
    /// QEMU binary to use.
    #[serde(default = "default_qemu_binary")]
    pub binary: String,

    /// Machine type.
    #[serde(default = "default_machine")]
    pub machine: String,

    /// Memory size in MB.
    #[serde(default = "default_memory")]
    pub memory: u32,

    /// Number of CPU cores.
    #[serde(default = "default_cores")]
    pub cores: u32,

    /// Enable KVM acceleration.
    #[serde(default = "default_true")]
    pub kvm: bool,

    /// Additional QEMU arguments.
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_qemu_binary() -> String {
    "qemu-system-x86_64".to_string()
}

fn default_machine() -> String {
    "q35".to_string()
}

fn default_memory() -> u32 {
    1024
}

fn default_cores() -> u32 {
    1
}

impl Default for QemuConfig {
    fn default() -> Self {
        Self {
            binary: "qemu-system-x86_64".to_string(),
            machine: "q35".to_string(),
            memory: 1024,
            cores: 1,
            kvm: true,
            extra_args: Vec::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// Test-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestConfig {
    /// Exit code that indicates test success.
    #[serde(rename = "success-exit-code")]
    pub success_exit_code: Option<i32>,

    /// Additional arguments for test runs.
    #[serde(default, rename = "extra-args")]
    pub extra_args: Vec<String>,

    /// Timeout for tests in seconds.
    pub timeout: Option<u64>,
}

/// Run-specific configuration (non-test).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunConfig {
    /// Additional arguments for normal runs.
    #[serde(default, rename = "extra-args")]
    pub extra_args: Vec<String>,

    /// Whether to use GUI display.
    #[serde(default)]
    pub gui: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        assert_eq!(config.boot.boot_type, BootType::Uefi);
        assert_eq!(config.bootloader.kind, BootloaderKind::None);
        assert!(config.bootloader.config_file.is_none());
        assert!(config.bootloader.extra_files.is_empty());
        assert_eq!(config.image.format, ImageFormat::Directory);
        assert!(config.image.output.is_none());
        assert_eq!(config.image.volume_label, "BOOT");
        assert_eq!(config.runner.kind, RunnerKind::Qemu);
        assert!(config.test.success_exit_code.is_none());
        assert!(config.test.extra_args.is_empty());
        assert!(config.test.timeout.is_none());
        assert!(!config.run.gui);
        assert!(config.run.extra_args.is_empty());
        assert!(config.variables.is_empty());
        assert!(!config.verbose);
    }

    #[test]
    fn test_config_deserialize_minimal() {
        let toml_str = r#"
        [boot]
        type = "uefi"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.boot.boot_type, BootType::Uefi);
        assert_eq!(config.bootloader.kind, BootloaderKind::None);
        assert_eq!(config.image.format, ImageFormat::Directory);
    }

    #[test]
    fn test_config_deserialize_full() {
        let toml_str = r#"
        verbose = true

        [boot]
        type = "hybrid"

        [bootloader]
        kind = "limine"
        config-file = "limine.conf"
        extra-files = ["extra.bin"]

        [bootloader.limine]
        version = "v8.4.0-binary"

        [image]
        format = "iso"
        output = "my-os.iso"
        volume_label = "MYOS"

        [runner]
        kind = "qemu"

        [runner.qemu]
        binary = "qemu-system-x86_64"
        memory = 2048
        cores = 2
        kvm = false

        [test]
        success-exit-code = 33
        timeout = 30
        extra-args = ["-device", "isa-debug-exit"]

        [run]
        gui = true
        extra-args = ["-serial", "stdio"]

        [variables]
        TIMEOUT = "5"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.boot.boot_type, BootType::Hybrid);
        assert_eq!(config.bootloader.kind, BootloaderKind::Limine);
        assert_eq!(
            config.bootloader.config_file,
            Some(PathBuf::from("limine.conf"))
        );
        assert_eq!(config.bootloader.limine.version, "v8.4.0-binary");
        assert_eq!(config.image.format, ImageFormat::Iso);
        assert_eq!(config.image.output, Some(PathBuf::from("my-os.iso")));
        assert_eq!(config.image.volume_label, "MYOS");
        assert_eq!(config.runner.qemu.memory, 2048);
        assert_eq!(config.runner.qemu.cores, 2);
        assert!(!config.runner.qemu.kvm);
        assert_eq!(config.test.success_exit_code, Some(33));
        assert_eq!(config.test.timeout, Some(30));
        assert!(config.run.gui);
        assert_eq!(config.variables.get("TIMEOUT").unwrap(), "5");
        assert!(config.verbose);
    }

    #[test]
    fn test_config_deserialize_bios_boot_type() {
        let toml_str = r#"
        [boot]
        type = "bios"

        [bootloader]
        kind = "grub"

        [image]
        format = "fat"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.boot.boot_type, BootType::Bios);
        assert_eq!(config.bootloader.kind, BootloaderKind::Grub);
        assert_eq!(config.image.format, ImageFormat::Fat);
    }

    #[test]
    fn test_config_deserialize_invalid_boot_type() {
        let toml_str = r#"
        [boot]
        type = "invalid"
        "#;
        let result: std::result::Result<Config, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_boot_type_needs_bios() {
        assert!(BootType::Bios.needs_bios());
        assert!(!BootType::Uefi.needs_bios());
        assert!(BootType::Hybrid.needs_bios());
    }

    #[test]
    fn test_boot_type_needs_uefi() {
        assert!(!BootType::Bios.needs_uefi());
        assert!(BootType::Uefi.needs_uefi());
        assert!(BootType::Hybrid.needs_uefi());
    }

    #[test]
    fn test_qemu_config_defaults() {
        let qemu = QemuConfig::default();
        assert_eq!(qemu.binary, "qemu-system-x86_64");
        assert_eq!(qemu.machine, "q35");
        assert_eq!(qemu.memory, 1024);
        assert_eq!(qemu.cores, 1);
        assert!(qemu.kvm);
        assert!(qemu.extra_args.is_empty());
    }

    #[test]
    fn test_limine_config_default_version() {
        let limine = LimineConfig::default();
        assert_eq!(limine.version, "v8.x-binary");
    }
}
