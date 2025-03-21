use std::fs::File;
use std::hash::{DefaultHasher, Hasher};
use std::io::Read;
use std::path::PathBuf;

use hadris_iso::{
    BootEntryOptions, BootOptions, BootSectionOptions, EmulationType, FileInput, FormatOptions,
    IsoImage, PartitionOptions, PlatformId, Strictness,
};

pub fn prepare_iso(
    root_dir: &PathBuf,
    iso_root: &PathBuf,
    iso_path: &PathBuf,
    target_exe_path: &PathBuf,
    target_dst_path: &PathBuf,
    config_path: &PathBuf,
    extra_files: &Vec<String>,
    limine_branch: &str,
    cmdline: &str,
) {
    let mut files_changed = false;

    let root_dir = PathBuf::from(root_dir);
    std::fs::create_dir_all(&iso_root).unwrap();

    let target_dst_path = iso_root.join(target_dst_path.file_name().unwrap());
    if !is_file_equal(&target_exe_path, &target_dst_path) {
        files_changed = true;
        std::fs::copy(&target_exe_path, &target_dst_path).expect(&format!(
            "failed to copy file {}",
            target_exe_path.to_string_lossy()
        ));
    }

    let config_dest_path = iso_root.join(config_path.strip_prefix(&root_dir).unwrap());
    if !is_file_equal(&config_path, &config_dest_path) {
        files_changed = true;
        // We need to format the contents of the config file with the
        // executable name
        let mut config_file_contents = std::fs::read_to_string(&config_path).unwrap();
        config_file_contents = config_file_contents.replace(
            "{{BINARY_NAME}}",
            &target_dst_path.file_name().unwrap().to_string_lossy(),
        );
        config_file_contents = config_file_contents.replace("{{CMDLINE}}", cmdline);
        std::fs::write(config_dest_path, config_file_contents).unwrap();
    }

    for file in extra_files.iter() {
        let file_path = root_dir.join(file);
        let file_dest_path = iso_root.join(file);
        if !is_file_equal(&file_path, &file_dest_path) {
            files_changed = true;
            let path = std::path::Path::new(file);
            std::fs::copy(path, file_dest_path).expect(&format!("failed to copy file {}", file));
        }
    }

    let plain_iso_file = std::path::Path::new(iso_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let limine_sys_file;
    let limine_bios_cd_file;
    let limine_uefi_cd_file;
    if limine_branch.split_once('-').unwrap().0 == "v4.x" {
        limine_sys_file = "limine.sys";
        limine_bios_cd_file = "limine-cd.bin";
        limine_uefi_cd_file = "limine-cd-efi.bin";
    } else {
        limine_sys_file = "limine-bios.sys";
        limine_bios_cd_file = "limine-bios-cd.bin";
        limine_uefi_cd_file = "limine-uefi-cd.bin";
    }

    // TODO: Make proper

    let limine_dir = root_dir.join("target/qemu-runner/limine");
    if !limine_dir.join(format!("{}_done", plain_iso_file)).exists() {
        std::fs::copy(
            limine_dir.join(limine_sys_file),
            iso_root.join(limine_sys_file),
        )
        .expect(&format!(
            "failed to copy file {}",
            limine_dir.join(limine_sys_file).to_string_lossy()
        ));
        std::fs::copy(
            limine_dir.join(limine_bios_cd_file),
            iso_root.join(limine_bios_cd_file),
        )
        .expect(&format!(
            "failed to copy file {}",
            limine_dir.join(limine_bios_cd_file).to_string_lossy()
        ));
        std::fs::copy(
            limine_dir.join(limine_uefi_cd_file),
            iso_root.join(limine_uefi_cd_file),
        )
        .expect(&format!(
            "failed to copy file {}",
            limine_dir.join(limine_uefi_cd_file).to_string_lossy()
        ));
        files_changed = true;
    }

    if !files_changed {
        println!("No files changed, skipping iso creation");
        return;
    }

    let entries = if cfg!(feature = "uefi") {
        vec![(
            BootSectionOptions {
                platform_id: PlatformId::UEFI,
            },
            BootEntryOptions {
                emulation: EmulationType::NoEmulation,
                // 0 means the size of the file
                load_size: 0,
                boot_image_path: limine_uefi_cd_file.to_string(),
                boot_info_table: false,
                grub2_boot_info: false,
            },
        )]
    } else {
        vec![]
    };

    let options = FormatOptions {
        strictness: Strictness::Strict,
        files: FileInput::from_fs(iso_root.clone()).unwrap(),
        // Only going to be used as CD/DVD boot, so we dont need MBR/GPT
        format: PartitionOptions::empty(),
        // We need to include the BIOS bootloader, because thats how El Torito boots
        boot: Some(BootOptions {
            write_boot_catalogue: true,
            default: BootEntryOptions {
                emulation: EmulationType::NoEmulation,
                load_size: 4,
                boot_image_path: limine_bios_cd_file.to_string(),
                boot_info_table: true,
                grub2_boot_info: false,
            },
            entries,
        }),
    };
    IsoImage::format_file(iso_path, options).unwrap();
}

fn hash_file(path: &PathBuf) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let mut hasher = DefaultHasher::new();
    let mut buffer = [0; 8192]; // Larger buffer for better performance

    loop {
        match file.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                hasher.write(&buffer[..n]);
            }
            Err(_) => return None,
        }
    }

    Some(hasher.finish())
}

fn is_file_equal(file1: &PathBuf, file2: &PathBuf) -> bool {
    // Quick rejection, if the files do not both exist
    if let (Ok(meta1), Ok(meta2)) = (file1.metadata(), file2.metadata()) {
        if meta1.len() != meta2.len() {
            // Again, quick rejection, if the files are different sizes
            return false;
        }
        match (hash_file(file1), hash_file(file2)) {
            (Some(hash1), Some(hash2)) => hash1 == hash2,
            _ => false,
        }
    } else {
        false
    }
}
