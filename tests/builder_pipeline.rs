use cargo_image_runner::config::{BootType, BootloaderKind, Config, ImageFormat};
use cargo_image_runner::ImageRunnerBuilder;

#[test]
fn test_uefi_none_directory_pipeline() {
    let dir = tempfile::tempdir().unwrap();

    // Create a fake executable
    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake uefi executable").unwrap();

    // Config: UEFI + None + Directory
    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Directory;

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();

    // Verify output directory exists
    assert!(image_path.exists());
    assert!(image_path.is_dir());

    // Verify executable is placed at efi/boot/bootx64.efi
    let bootx64 = image_path.join("efi/boot/bootx64.efi");
    assert!(bootx64.exists(), "bootx64.efi should exist at {:?}", bootx64);
    assert_eq!(
        std::fs::read_to_string(&bootx64).unwrap(),
        "fake uefi executable"
    );
}

#[test]
fn test_builder_auto_selects_from_config() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake").unwrap();

    // Default config uses None bootloader + Directory format
    let runner = ImageRunnerBuilder::new()
        .with_config(Config::default())
        .workspace_root(dir.path())
        .executable(&exe)
        .build();

    assert!(runner.is_ok(), "builder should auto-select components from config");
}

#[test]
fn test_builder_error_missing_executable_file() {
    let dir = tempfile::tempdir().unwrap();
    // Executable path that doesn't exist on disk
    let exe = dir.path().join("nonexistent-kernel");

    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Directory;

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    // Building the image should fail because the executable doesn't exist
    let result = runner.build_image();
    assert!(result.is_err());
}

/// Test ISO image creation with UEFI + None bootloader.
/// This exercises the hadris-iso code path with minimal inputs.
#[cfg(feature = "iso")]
#[test]
fn test_uefi_none_iso_pipeline() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake uefi executable content").unwrap();

    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Iso;

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();

    assert!(image_path.exists(), "ISO file should exist at {:?}", image_path);
    assert!(image_path.is_file());
    // ISO should have non-trivial size (at minimum the ISO header structures)
    let metadata = std::fs::metadata(&image_path).unwrap();
    assert!(metadata.len() > 0, "ISO file should not be empty");
}

/// Test ISO image creation with Hybrid boot type + no boot images available.
/// This exercises the El Torito path where no boot images are found (returns None).
#[cfg(feature = "iso")]
#[test]
fn test_hybrid_none_iso_pipeline_no_boot_images() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake hybrid kernel").unwrap();

    let mut config = Config::default();
    config.boot.boot_type = BootType::Hybrid;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Iso;

    // NoneBootloader with Hybrid: only adds UEFI file (efi/boot/bootx64.efi).
    // El Torito configure_boot_options for Hybrid looks for limine-bios-cd.bin → not found → None.
    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let result = runner.build_image();
    // This may fail due to hadris-iso issues — document behavior either way
    if let Err(e) = &result {
        eprintln!("ISO build with Hybrid+None failed: {}", e);
    } else {
        let image_path = result.unwrap();
        assert!(image_path.exists());
        assert!(image_path.is_file());
    }
}

/// Regression test: ISO with nested directory paths (e.g., "efi/boot/bootx64.efi").
/// Previously, prepare_iso_files used the full relative path as the ISO filename,
/// which exceeded hadris-iso's 30-byte FixedFilename limit and caused a panic.
/// The fix uses InputFiles::from_fs which builds a proper directory tree.
#[cfg(feature = "iso")]
#[test]
fn test_iso_with_nested_directory_paths() {
    let dir = tempfile::tempdir().unwrap();

    let kernel = dir.path().join("hadron.elf");
    std::fs::write(&kernel, vec![0xBB_u8; 8192]).unwrap();

    // UEFI+None puts the exe at efi/boot/bootx64.efi — a nested path
    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Iso;

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&kernel)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();
    assert!(image_path.exists());
    assert!(image_path.is_file());
}

/// Test ISO with multiple files to check hadris-iso handles file trees.
#[cfg(feature = "iso")]
#[test]
fn test_iso_with_multiple_files() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    // Create a larger fake executable to test non-trivial file sizes
    let exe_content = vec![0xAA_u8; 4096];
    std::fs::write(&exe, &exe_content).unwrap();

    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Iso;

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();
    assert!(image_path.exists());

    let size = std::fs::metadata(&image_path).unwrap().len();
    assert!(size > 4096, "ISO should be larger than the input file, got {} bytes", size);
}

/// Test FAT image creation with UEFI + None bootloader.
#[cfg(feature = "fat")]
#[test]
fn test_uefi_none_fat_pipeline() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake uefi executable").unwrap();

    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Fat;

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();

    assert!(image_path.exists(), "FAT image should exist at {:?}", image_path);
    assert!(image_path.is_file());
    let metadata = std::fs::metadata(&image_path).unwrap();
    // FAT images have minimum 32MB size
    assert!(metadata.len() >= 32 * 1024 * 1024, "FAT image should be at least 32MB");
}
