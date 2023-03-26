use std::fs::File;
use std::io::Read;
use std::process::{Command, exit, Stdio};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RunnerMetadata {
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
	test_success_exit_code: u32
}

#[derive(Debug, Deserialize)]
struct PackageMetadata {
	qemu_runner: RunnerMetadata
}

fn is_file_equal(file1: &str, file2: &str) -> bool {
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
				_ => return false
			};

			if vec1 != vec2 {
				return false;
			}
		}

		true
	}
	else {
		false
	}
}

fn prepare_iso(
	iso_root: &str,
	iso_file: &str,
	target_exe_path: &str,
	target_dst_name: &str,
	extra_files: &Vec<String>,
	limine_branch: &str) {
	std::fs::create_dir_all(iso_root).unwrap();

	let mut files_changed = false;

	let target_dest_path = format!("{}/{}", iso_root, target_dst_name);

	if !is_file_equal(target_exe_path, &target_dest_path) {
		files_changed = true;
		std::fs::copy(target_exe_path, &target_dest_path)
			.expect(&format!("failed to copy file {}", target_exe_path));
	}

	for file in extra_files.iter() {
		if !is_file_equal(file, format!("{}/{}", iso_root, file).as_str()) {
			files_changed = true;
			let path = std::path::Path::new(file);
			std::fs::copy(path, format!("{}/{}", iso_root, path.file_name().unwrap().to_str().unwrap()))
				.expect(&format!("failed to copy file {}", file));
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
				"target/limine"
			]).spawn().expect("failed to git clone limine");
		let status = git_cmd.wait().unwrap();
		if !status.success() {
			panic!("failed to git clone limine");
		}

		files_changed = true;

		std::fs::write("target/limine/done", "")
			.expect("failed to write to target/limine/done");
	}

	let plain_iso_file = std::path::Path::new(iso_file).file_name().unwrap().to_str().unwrap();

	if !std::path::Path::new(&format!("target/limine/{}_done", plain_iso_file)).exists() {
		std::fs::copy("target/limine/limine.sys", &format!("{}/{}", iso_root, "limine.sys"))
			.expect(&format!("failed to copy file {}", &format!("{}/{}", iso_root, "limine.sys")));
		std::fs::copy("target/limine/limine-cd.bin", &format!("{}/{}", iso_root, "limine-cd.bin"))
			.expect(&format!("failed to copy file {}", &format!("{}/{}", iso_root, "limine-cd.bin")));
		std::fs::copy("target/limine/limine-cd-efi.bin", &format!("{}/{}", iso_root, "limine-cd-efi.bin"))
			.expect(&format!("failed to copy file {}", &format!("{}/{}", iso_root, "limine-cd-efi.bin")));
		files_changed = true;
	}

	if files_changed {
		let xorriso_cmd = Command::new("xorriso")
			.args(vec![
				"-as", "mkisofs",
				"-b", "limine-cd.bin",
				"-no-emul-boot",
				"-boot-load-size", "4",
				"-boot-info-table",
				"--efi-boot", "limine-cd-efi.bin",
				"--efi-boot-part",
				"--efi-boot-image",
				"--protective-msdos-label",
				iso_root,
				"-o", iso_file,
				"-quiet"
			]).stdout(Stdio::piped()).stderr(Stdio::piped())
			.spawn().expect("failed to start xorriso");
		let status = xorriso_cmd.wait_with_output().unwrap();
		if !status.status.success() {
			panic!("failed to create iso file using xorriso: {}", String::from_utf8(status.stderr).unwrap());
		}
	}
}

fn main() {
	let args: Vec<_> = std::env::args().collect();
	let mut args_iter = args.iter().skip(2);

	let target_exe_path = args_iter.next().expect("expected path to target executable");
	let mut args_iter = args_iter.skip_while(|val| !val.starts_with("--manifest-path"));

	let mut cmd = cargo_metadata::MetadataCommand::new();

	match args_iter.next() {
		Some(p) if p == "--manifest-path" => {
			cmd.manifest_path(args_iter.next().unwrap());
		}
		Some(p) => {
			cmd.manifest_path(p.trim_start_matches("--manifest-path"));
		}
		None => {}
	}

	let metadata = cmd.exec().unwrap();

	let root_package = metadata.root_package().expect("no root package found");

	let mut data: PackageMetadata = serde_json::from_value(root_package.metadata.clone())
		.expect("no [package.manifest.qemu_runner] entry specified");

	let mut target_dest_file = std::path::Path::new(target_exe_path).file_name().unwrap().to_str().unwrap();

	let mut is_test = false;
	if let Some((start, end)) = target_dest_file.rsplit_once('-') {
		if u64::from_str_radix(end, 16).is_ok() {
			target_dest_file = start;
			is_test = true;
		}
	}

	if !is_test {
		prepare_iso(
			"target/iso_root",
			"target/image.iso",
			target_exe_path,
			target_dest_file,
			&data.qemu_runner.extra_files,
			&data.qemu_runner.limine_branch);
		for arg in data.qemu_runner.run_command.iter_mut() {
			*arg = arg.replace("{}", "target/image.iso");
		}
	}
	else {
		prepare_iso(
			"target/test_iso_root",
			"target/test_image.iso",
			target_exe_path,
			target_dest_file,
			&data.qemu_runner.extra_files,
			&data.qemu_runner.limine_branch);
		for arg in data.qemu_runner.run_command.iter_mut() {
			*arg = arg.replace("{}", "target/test_image.iso");
		}
	}

	let run_exe = data.qemu_runner.run_command.get(0).expect("no run command provided");
	let mut run_command = Command::new(run_exe);
	run_command.args(data.qemu_runner.run_command.iter().skip(1));
	if is_test {
		run_command.args(data.qemu_runner.test_args);
	}
	else {
		run_command.args(data.qemu_runner.run_args);
	}

	let mut run_command = run_command.spawn().expect("run command failed");
	let status = run_command.wait().unwrap();
	if !is_test {
		if !status.success() {
			exit(status.code().unwrap_or(1));
		}
	}
	else {
		let code = status.code().unwrap_or(i32::MAX);
		if code as u32 != data.qemu_runner.test_success_exit_code {
			exit(code);
		}
	}
}
