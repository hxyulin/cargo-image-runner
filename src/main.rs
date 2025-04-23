use bootloader::prepare_bootloader;
use iso::prepare_iso;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, exit};

mod bootloader;
mod iso;

/// An enum representing the boot type to use
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
    #[serde(default)]
    #[serde(rename = "extra-files")]
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
    #[serde(rename = "boot-type")]
    #[serde(default)]
    boot_type: BootType,
    /// The kernel command line to use
    #[serde(default)]
    cmdline: String,
}

#[derive(Debug, Deserialize)]
struct PackageMetadata {
    qemu_runner: RunnerMetadata,
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

    // TODO: This gives a wrong error message if the metadata is not found
    let mut data: PackageMetadata = serde_json::from_value(package.metadata.clone())
        .unwrap_or_else(|_| {
            serde_json::from_value(metadata.workspace_metadata.clone())
                .expect("no [package.metadata.qemu_runner] entry specified")
        });

    #[cfg(not(feature = "bios"))]
    if data.qemu_runner.boot_type == BootType::Bios {
        panic!("bios boot type is not supported, enable the `bios` feature");
    }
    #[cfg(not(feature = "uefi"))]
    if data.qemu_runner.boot_type == BootType::Uefi {
        panic!("uefi boot type is not supported, enable the `uefi` feature");
    }

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

    let root_dir = PathBuf::from(root_dir);
    let file_dir = root_dir.join("target/qemu-runner");
    prepare_bootloader(&data.qemu_runner.limine_branch, &file_dir);

    let target_exe_path = root_dir.join(target_exe_path);
    let target_dest_file = root_dir.join(target_dest_file);
    let config_path = root_dir.join(data.qemu_runner.config_file.as_str());

    let (iso_dir, iso_path) = if is_test {
        let target_name = target_exe_path.to_string_lossy();
        let target_name = target_name.rsplit_once('/').unwrap().1;
        let tests_dir = file_dir.join("tests");
        let iso_path = tests_dir.join(format!("{}.iso", target_name));
        let iso_dir = tests_dir.join(format!("{}_isoroot", target_name));
        (iso_dir, iso_path)
    } else {
        let iso_path = file_dir.join("image.iso");
        let iso_dir = file_dir.join("iso_root");
        (iso_dir, iso_path)
    };

    prepare_iso(
        &root_dir,
        &iso_dir,
        &iso_path,
        &target_exe_path,
        &target_dest_file,
        &config_path,
        &data.qemu_runner.extra_files,
        &data.qemu_runner.limine_branch,
        &data.qemu_runner.cmdline,
    );
    for arg in data.qemu_runner.run_command.iter_mut() {
        *arg = arg.replace("{}", &iso_path.to_string_lossy());
    }

    let run_exe = data
        .qemu_runner
        .run_command
        .first()
        .expect("no run command provided");
    let mut run_command = Command::new(run_exe);

    if cfg!(feature = "uefi") && data.qemu_runner.boot_type == BootType::Uefi {
        let ovmf =
            ovmf_prebuilt::Prebuilt::fetch(ovmf_prebuilt::Source::LATEST, "target/ovmf").unwrap();
        let code = ovmf.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Code);
        let vars = ovmf.get_file(ovmf_prebuilt::Arch::X64, ovmf_prebuilt::FileType::Vars);

        run_command
            .arg("-drive")
            .arg(format!(
                "if=pflash,format=raw,readonly=on,file={}",
                code.display()
            ))
            .arg("-drive")
            .arg(format!("if=pflash,format=raw,file={}", vars.display()));
    }

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
