use cargo_image_runner::config::{BootType, BootloaderKind, ConfigLoader, ImageFormat};

#[test]
fn test_full_config_parsing() {
    let toml_str = r#"
[boot]
type = "hybrid"

[bootloader]
kind = "limine"
config-file = "limine.conf"

[bootloader.limine]
version = "v8.4.0-binary"

[image]
format = "iso"
volume_label = "MYOS"

[runner]
kind = "qemu"

[runner.qemu]
memory = 2048
cores = 2
kvm = false

[test]
success-exit-code = 33
timeout = 30
extra-args = ["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"]

[run]
gui = true
extra-args = ["-serial", "stdio"]

[variables]
TIMEOUT = "5"
KERNEL_CMDLINE = "quiet"
"#;
    let config: cargo_image_runner::Config = toml::from_str(toml_str).unwrap();

    assert_eq!(config.boot.boot_type, BootType::Hybrid);
    assert_eq!(config.bootloader.kind, BootloaderKind::Limine);
    assert_eq!(
        config.bootloader.config_file,
        Some(std::path::PathBuf::from("limine.conf"))
    );
    assert_eq!(config.bootloader.limine.version, "v8.4.0-binary");
    assert_eq!(config.image.format, ImageFormat::Iso);
    assert_eq!(config.image.volume_label, "MYOS");
    assert_eq!(config.runner.qemu.memory, 2048);
    assert_eq!(config.runner.qemu.cores, 2);
    assert!(!config.runner.qemu.kvm);
    assert_eq!(config.test.success_exit_code, Some(33));
    assert_eq!(config.test.timeout, Some(30));
    assert!(config.run.gui);
    assert_eq!(config.variables.get("TIMEOUT").unwrap(), "5");
    assert_eq!(config.variables.get("KERNEL_CMDLINE").unwrap(), "quiet");
}

#[test]
fn test_config_loader_standalone_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("image-runner.toml");
    std::fs::write(
        &config_path,
        r#"
[boot]
type = "uefi"

[bootloader]
kind = "none"

[image]
format = "directory"
"#,
    )
    .unwrap();

    let (config, workspace_root) = ConfigLoader::new()
        .no_cargo_metadata()
        .workspace_root(dir.path())
        .config_file(&config_path)
        .load()
        .unwrap();

    assert_eq!(config.boot.boot_type, BootType::Uefi);
    assert_eq!(config.bootloader.kind, BootloaderKind::None);
    assert_eq!(config.image.format, ImageFormat::Directory);
    assert_eq!(workspace_root, dir.path());
}

#[test]
fn test_config_loader_overrides_defaults() {
    let dir = tempfile::tempdir().unwrap();

    // Config file that overrides defaults
    let config_path = dir.path().join("override.toml");
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
B = "from_override"
C = "from_override"
"#,
    )
    .unwrap();

    // The loader merges defaults with the config file
    let (config, _) = ConfigLoader::new()
        .no_cargo_metadata()
        .workspace_root(dir.path())
        .config_file(&config_path)
        .load()
        .unwrap();

    // Override values take precedence over defaults
    assert_eq!(config.boot.boot_type, BootType::Hybrid);
    assert_eq!(config.bootloader.kind, BootloaderKind::Limine);
    assert_eq!(config.image.format, ImageFormat::Iso);
    assert_eq!(config.variables.get("B").unwrap(), "from_override");
    assert_eq!(config.variables.get("C").unwrap(), "from_override");

    // Defaults are preserved where not overridden
    assert_eq!(config.runner.qemu.memory, 1024); // default
}

#[test]
fn test_config_defaults_without_file() {
    let dir = tempfile::tempdir().unwrap();

    let (config, _) = ConfigLoader::new()
        .no_cargo_metadata()
        .workspace_root(dir.path())
        .load()
        .unwrap();

    // All defaults
    assert_eq!(config.boot.boot_type, BootType::Uefi);
    assert_eq!(config.bootloader.kind, BootloaderKind::None);
    assert_eq!(config.image.format, ImageFormat::Directory);
    assert!(config.variables.is_empty());
}
