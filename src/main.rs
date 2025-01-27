use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum BootType {
    #[default]
    #[serde(rename = "bios")]
    Bios,
    #[serde(rename = "uefi")]
    Uefi,
}

#[derive(Debug, Deserialize)]
struct RunnerMetadata {
    #[serde(rename = "config-file")]
    config_file: String,
    #[serde(rename = "extra-files")]
    #[serde(default)]
    extra_files: Vec<String>,
    #[serde(rename = "limine-branch")]
    limine_branch: String,
    #[serde(rename = "run-command")]
    run_command: Vec<String>,
    #[serde(rename = "test-args")]
    #[serde(default)]
    test_args: Vec<String>,
    #[serde(rename = "run-args")]
    #[serde(default)]
    run_args: Vec<String>,
    #[serde(rename = "test-success-exit-code")]
    #[serde(default)]
    test_success_exit_code: u32,
    //#[serde(rename = "boot-type")]
    //#[serde(default)]
    //boot_type: BootType,
}

#[derive(Debug, Deserialize)]
struct PackageMetadata {
    qemu_runner: RunnerMetadata,
}

fn is_file_equal(file1: &PathBuf, file2: &PathBuf) -> bool {
    let mut f1 = File::open(file1).unwrap();
    let f2 = File::open(file2);

    if let Ok(mut f2) = f2 {
        if f1.metadata().unwrap().len() != f2.metadata().unwrap().len() {
            return false;
        }

        let mut vec1 = [0u8; 512];
        let mut vec2 = [0u8; 512];
        let mut count = 1;
        while count > 0 {
            if f1.read(&mut vec2).is_err() {
                return false;
            }

            count = match f2.read(&mut vec1) {
                Ok(count) => count,
                _ => return false,
            };

            if vec1 != vec2 {
                return false;
            }
        }

        true
    } else {
        false
    }
}

fn prepare_iso(
    root_dir: &str,
    iso_root: &str,
    iso_file: &str,
    target_exe_path: &str,
    target_dst_name: &str,
    config_file: &str,
    extra_files: &Vec<String>,
    limine_branch: &str,
) {
    let mut files_changed = false;

    let root_dir = PathBuf::from(root_dir);
    let iso_root = root_dir.join(iso_root);
    std::fs::create_dir_all(&iso_root).unwrap();
    let target_dest_path = iso_root.join(target_dst_name);
    let target_exe_path = root_dir.join(target_exe_path);

    if !is_file_equal(&target_exe_path, &target_dest_path) {
        files_changed = true;
        std::fs::copy(&target_exe_path, &target_dest_path).expect(&format!(
            "failed to copy file {}",
            target_exe_path.to_string_lossy()
        ));
    }

    let config_path = root_dir.join(config_file);
    let config_dest_path = iso_root.join(config_file);
    if !is_file_equal(&config_path, &config_dest_path) {
        files_changed = true;
        // We need to do something, we need to format the contents of the config file with the
        // exeucutable name as the first argument
        let mut config_file_contents = std::fs::read_to_string(&config_path).unwrap();
        config_file_contents = config_file_contents.replace("{{BINARY_NAME}}", target_dst_name);
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

    if !std::path::Path::new("target/limine/done").exists() {
        std::fs::remove_dir_all("target/limine").ok();
        let mut git_cmd = Command::new("git")
            .args(vec![
                "clone",
                "https://github.com/limine-bootloader/limine",
                "-b",
                limine_branch,
                "--depth=1",
                "target/limine",
            ])
            .spawn()
            .expect("failed to git clone limine");
        let status = git_cmd.wait().unwrap();
        if !status.success() {
            panic!("failed to git clone limine");
        }

        files_changed = true;

        std::fs::write("target/limine/done", "").expect("failed to write to target/limine/done");
    }

    let plain_iso_file = std::path::Path::new(iso_file)
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

    if !std::path::Path::new(&format!("target/limine/{}_done", plain_iso_file)).exists() {
        let limine_path = root_dir.join("target/limine");
        std::fs::copy(
            limine_path.join(limine_sys_file),
            iso_root.join(limine_sys_file),
        )
        .expect(&format!(
            "failed to copy file {}",
            limine_path.join(limine_sys_file).to_string_lossy()
        ));
        std::fs::copy(
            limine_path.join(limine_bios_cd_file),
            iso_root.join(limine_bios_cd_file),
        )
        .expect(&format!(
            "failed to copy file {}",
            limine_path.join(limine_bios_cd_file).to_string_lossy()
        ));
        std::fs::copy(
            limine_path.join(limine_uefi_cd_file),
            iso_root.join(limine_uefi_cd_file),
        )
        .expect(&format!(
            "failed to copy file {}",
            limine_path.join(limine_uefi_cd_file).to_string_lossy()
        ));
        files_changed = true;
    }

    if files_changed {
        let xorriso_cmd = Command::new("xorriso")
            .args(vec![
                "-as",
                "mkisofs",
                "-b",
                limine_bios_cd_file,
                "-no-emul-boot",
                "-boot-load-size",
                "4",
                "-boot-info-table",
                "--efi-boot",
                limine_uefi_cd_file,
                "--efi-boot-part",
                "--efi-boot-image",
                "--protective-msdos-label",
                &iso_root.to_string_lossy(),
                "-o",
                iso_file,
                "-quiet",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start xorriso");
        let status = xorriso_cmd.wait_with_output().unwrap();
        if !status.status.success() {
            panic!(
                "failed to create iso file using xorriso: {}",
                String::from_utf8(status.stderr).unwrap()
            );
        }
    }
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut args_iter = args.iter().skip(2);

    //let target = std::env::var("TARGET").unwrap_or("x86_64".to_string());
    let manifest_path = std::env::var("CARGO_MANIFEST_PATH").ok();
    let pkg_name = std::env::var("CARGO_PKG_NAME").ok();

    let target_exe_path = args_iter
        .next()
        .expect("expected path to target executable");

    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }

    let metadata = cmd.exec().unwrap();

    let package = match pkg_name {
        Some(pkg_name) => metadata
            .packages
            .iter()
            .find(|p| p.name == pkg_name)
            .unwrap(),
        None => metadata.root_package().unwrap(),
    };
    let root_dir = metadata.workspace_root.as_str();

    let mut data: PackageMetadata = serde_json::from_value(package.metadata.clone())
        .expect("no [package.metadata.qemu_runner] entry specified");

    let mut target_dest_file = std::path::Path::new(target_exe_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let mut is_test = false;
    if let Some((start, end)) = target_dest_file.rsplit_once('-') {
        if u64::from_str_radix(end, 16).is_ok() {
            target_dest_file = start;
            is_test = true;
        }
    }

    if !is_test {
        prepare_iso(
            root_dir,
            "target/iso_root",
            "target/image.iso",
            target_exe_path,
            target_dest_file,
            data.qemu_runner.config_file.as_str(),
            &data.qemu_runner.extra_files,
            &data.qemu_runner.limine_branch,
        );
        for arg in data.qemu_runner.run_command.iter_mut() {
            *arg = arg.replace("{}", "target/image.iso");
        }
    } else {
        prepare_iso(
            root_dir,
            "target/test_iso_root",
            "target/test_image.iso",
            target_exe_path,
            target_dest_file,
            data.qemu_runner.config_file.as_str(),
            &data.qemu_runner.extra_files,
            &data.qemu_runner.limine_branch,
        );
        for arg in data.qemu_runner.run_command.iter_mut() {
            *arg = arg.replace("{}", "target/test_image.iso");
        }
    }

    let run_exe = data
        .qemu_runner
        .run_command
        .first()
        .expect("no run command provided");
    let mut run_command = Command::new(run_exe);
    run_command.args(data.qemu_runner.run_command.iter().skip(1));
    if is_test {
        run_command.args(data.qemu_runner.test_args);
    } else {
        run_command.args(data.qemu_runner.run_args);
    }

    let mut run_command = run_command.spawn().expect("run command failed");
    let status = run_command.wait().unwrap();
    if !is_test {
        if !status.success() {
            exit(status.code().unwrap_or(1));
        }
    } else {
        let code = status.code().unwrap_or(i32::MAX);
        if code as u32 != data.qemu_runner.test_success_exit_code {
            exit(code);
        }
    }
}
