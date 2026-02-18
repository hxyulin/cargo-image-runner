//! Integration tests for standalone library usage (no cargo_metadata required).

use cargo_image_runner::{builder, Config};

/// Test the full standalone workflow: Config::from_toml_str -> builder -> build_image.
#[test]
fn test_standalone_from_toml_str() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake uefi executable").unwrap();

    let config = Config::from_toml_str(
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

    assert_eq!(config.boot.boot_type, cargo_image_runner::BootType::Uefi);
    assert_eq!(
        config.bootloader.kind,
        cargo_image_runner::BootloaderKind::None
    );
    assert_eq!(
        config.image.format,
        cargo_image_runner::ImageFormat::Directory
    );

    let runner = builder()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();
    assert!(image_path.exists());
    assert!(image_path.is_dir());

    let bootx64 = image_path.join("efi/boot/bootx64.efi");
    assert!(bootx64.exists());
    assert_eq!(
        std::fs::read_to_string(&bootx64).unwrap(),
        "fake uefi executable"
    );
}

/// Test Config::from_toml_file for loading from a standalone file.
#[test]
fn test_standalone_from_toml_file() {
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

        [variables]
        KERNEL_NAME = "test-kernel"
        "#,
    )
    .unwrap();

    let config = Config::from_toml_file(&config_path).unwrap();
    assert_eq!(config.boot.boot_type, cargo_image_runner::BootType::Uefi);
    assert_eq!(config.variables.get("KERNEL_NAME").unwrap(), "test-kernel");
}

/// Test that Config::from_toml_str with defaults works (minimal TOML).
#[test]
fn test_standalone_minimal_config() {
    let config = Config::from_toml_str("").unwrap();
    assert_eq!(config.boot.boot_type, cargo_image_runner::BootType::Uefi);
    assert_eq!(
        config.bootloader.kind,
        cargo_image_runner::BootloaderKind::None
    );
    assert_eq!(
        config.image.format,
        cargo_image_runner::ImageFormat::Directory
    );
}

/// Test error on invalid TOML input.
#[test]
fn test_standalone_invalid_toml() {
    let result = Config::from_toml_str("not valid { toml [[[");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("TOML"));
}

/// Test error on missing config file.
#[test]
fn test_standalone_missing_file() {
    let result = Config::from_toml_file("/nonexistent/path/config.toml");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("config file"));
}
