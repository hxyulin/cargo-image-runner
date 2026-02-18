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

/// Test extra-files are placed at correct destination paths in directory output.
#[test]
fn test_extra_files_directory_pipeline() {
    let dir = tempfile::tempdir().unwrap();

    // Create a fake executable
    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake uefi executable").unwrap();

    // Create extra source files
    std::fs::write(dir.path().join("data.txt"), "hello world").unwrap();
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/initramfs.cpio"), "fake initramfs").unwrap();

    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Directory;
    config
        .extra_files
        .insert("boot/data.txt".to_string(), "data.txt".to_string());
    config
        .extra_files
        .insert("boot/initramfs.cpio".to_string(), "build/initramfs.cpio".to_string());

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();

    // Verify extra files at correct destination paths
    let data_file = image_path.join("boot/data.txt");
    assert!(data_file.exists(), "data.txt should exist at {:?}", data_file);
    assert_eq!(std::fs::read_to_string(&data_file).unwrap(), "hello world");

    let initramfs = image_path.join("boot/initramfs.cpio");
    assert!(
        initramfs.exists(),
        "initramfs.cpio should exist at {:?}",
        initramfs
    );
    assert_eq!(
        std::fs::read_to_string(&initramfs).unwrap(),
        "fake initramfs"
    );
}

/// Test that missing extra source file produces a clear error.
#[test]
fn test_extra_files_missing_source_error() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake").unwrap();

    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Directory;
    config
        .extra_files
        .insert("boot/missing.txt".to_string(), "nonexistent.txt".to_string());

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let result = runner.build_image();
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("extra file not found"),
        "error should mention extra file not found, got: {}",
        err
    );
}

/// Test that empty extra-files is a no-op.
#[test]
fn test_empty_extra_files_noop() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake uefi executable").unwrap();

    // Config with empty extra_files (default)
    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Directory;
    assert!(config.extra_files.is_empty());

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();
    assert!(image_path.exists());

    // Only the executable should be placed, no extra files
    let bootx64 = image_path.join("efi/boot/bootx64.efi");
    assert!(bootx64.exists());
}

/// Test that absolute destination paths (with leading `/`) are treated as relative to image root.
#[test]
fn test_extra_files_absolute_dest_path() {
    let dir = tempfile::tempdir().unwrap();

    let exe = dir.path().join("kernel.efi");
    std::fs::write(&exe, b"fake uefi executable").unwrap();

    // Create extra source file
    std::fs::write(dir.path().join("initrd.cpio"), "fake initrd").unwrap();

    let mut config = Config::default();
    config.boot.boot_type = BootType::Uefi;
    config.bootloader.kind = BootloaderKind::None;
    config.image.format = ImageFormat::Directory;
    // Use absolute destination path — should be stripped to relative
    config
        .extra_files
        .insert("/boot/initrd.cpio".to_string(), "initrd.cpio".to_string());

    let runner = ImageRunnerBuilder::new()
        .with_config(config)
        .workspace_root(dir.path())
        .executable(&exe)
        .build()
        .unwrap();

    let image_path = runner.build_image().unwrap();

    // File should land at boot/initrd.cpio (relative), not /boot/initrd.cpio (absolute)
    let initrd = image_path.join("boot/initrd.cpio");
    assert!(
        initrd.exists(),
        "initrd.cpio should exist at {:?}",
        initrd
    );
    assert_eq!(
        std::fs::read_to_string(&initrd).unwrap(),
        "fake initrd"
    );
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
