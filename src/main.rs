use bootloader::prepare_bootloader;
use iso::prepare_iso;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

mod bootloader;
mod config;
mod iso;
use config::{BootType, PackageMetadata, default_config};

use crate::config::ImageRunnerConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    String(String),
}

impl<T> From<T> for Value
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match value.as_ref() {
            "true" => Self::Bool(true),
            "false" => Self::Bool(false),
            str => Self::String(str.to_string()),
        }
    }
}

#[cfg(test)]
#[test]
fn test_value_from_str() {
    let tr: Value = "true".into();
    let fl: Value = "false".into();
    let other: Value = "other".into();
    assert_eq!(tr, Value::Bool(true));
    assert_eq!(fl, Value::Bool(false));
    assert_eq!(other, Value::String("other".to_string()))
}

impl Value {
    pub fn parse_pair(str: &str) -> (String, Value) {
        // Default value would be true, if you use this syntax
        let (key, value) = str.split_once('=').unwrap_or((str, "true"));
        (key.to_string(), value.into())
    }

    pub fn as_string(self) -> Result<String, ()> {
        match self {
            Self::String(str) => Ok(str),
            _ => Err(()),
        }
    }
}

struct ParseCtx {
    config: ImageRunnerConfig,
    target_src: PathBuf,
    target_dst: PathBuf,
    root_dir: PathBuf,
    file_dir: PathBuf,
    config_path: PathBuf,
    is_test: bool,
}

impl ParseCtx {
    pub fn new(config: ImageRunnerConfig, target_src: PathBuf, root_dir: PathBuf) -> ParseCtx {
        #[cfg(not(feature = "bios"))]
        if config.boot_type == BootType::Bios {
            panic!("BIOS boot type is not supported, enable the `bios` feature for this crate");
        }
        #[cfg(not(feature = "uefi"))]
        if config.boot_type == BootType::Uefi {
            panic!("UEFI boot type is not supported, enable the `uefi` feature for this crate");
        }

        let file_dir = root_dir.join("target/image-runner");

        let target_src = root_dir.join(target_src);

        let mut target_name = Path::new(&target_src)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();

        let mut is_test = false;
        if let Some((start, end)) = target_name.rsplit_once('-') {
            if u64::from_str_radix(end, 16).is_ok() {
                target_name = start;
                is_test = true;
            }
        }

        let target_dst = root_dir.join(target_name);

        let config_path = root_dir.join(config.config_file.as_str());

        Self {
            config,
            target_src,
            target_dst,
            root_dir,
            file_dir,
            config_path,
            is_test,
        }
    }

    fn prepare_bootloader(&self) {
        prepare_bootloader(&self.config.limine_branch, &self.file_dir);
    }

    fn prepare_iso(&mut self) {
        let (iso_dir, iso_path) = if self.is_test {
            let target_name = self.target_src.to_string_lossy();
            let target_name = target_name.rsplit_once('/').unwrap().1;
            let tests_dir = self.file_dir.join("tests");
            let iso_path = tests_dir.join(format!("{}.iso", target_name));
            let iso_dir = tests_dir.join(format!("{}_isoroot", target_name));
            (iso_dir, iso_path)
        } else {
            let iso_path = self.file_dir.join("image.iso");
            let iso_dir = self.file_dir.join("iso_root");
            (iso_dir, iso_path)
        };

        prepare_iso(
            &self.root_dir,
            &iso_dir,
            &iso_path,
            &self.target_src,
            &self.target_dst,
            &self.config_path,
            &self.config.extra_files,
            &self.config.limine_branch,
            &self.config.cmdline,
        );
        for arg in self.config.run_command.iter_mut() {
            *arg = arg.replace("{}", &iso_path.to_string_lossy());
            for (k, v) in self.config.vars.iter() {
                *arg = arg.replace(&format!("${}", k), v);
            }
        }
        for arg in self.config.run_args.iter_mut() {
            for (k, v) in self.config.vars.iter() {
                *arg = arg.replace(&format!("${}", k), v);
            }
        }

        for arg in self.config.test_args.iter_mut() {
            for (k, v) in self.config.vars.iter() {
                *arg = arg.replace(&format!("${}", k), v);
            }
        }
    }

    fn run(self) {
        let run_cmd = self
            .config
            .run_command
            .first()
            .expect("no run command provided");
        let mut run_command = Command::new(run_cmd);

        if cfg!(feature = "uefi") && self.config.boot_type == BootType::Uefi {
            println!("Fetching OVMF firmware...");
            let ovmf = ovmf_prebuilt::Prebuilt::fetch(ovmf_prebuilt::Source::LATEST, "target/ovmf")
                .unwrap();
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

        run_command.args(self.config.run_command.iter().skip(1));
        if self.is_test {
            run_command.args(self.config.test_args);
        } else {
            run_command.args(self.config.run_args);
        }

        let mut run_command = run_command.spawn().expect("run command failed");
        let status = run_command.wait().unwrap();
        if !self.is_test {
            if !status.success() {
                exit(status.code().unwrap_or(1));
            }
        } else {
            let code = status.code().unwrap_or(i32::MAX);
            if code as u32 != self.config.test_success_exit_code {
                exit(code);
            }
        }
    }
}

fn main() {
    let mut args_iter = std::env::args().skip(2);

    // We allow passing arguments as key value pairs such as
    //let target = std::env::var("TARGET").unwrap_or("x86_64".to_string());
    let manifest_path = std::env::var("CARGO_MANIFEST_PATH").ok();
    let pkg_name = std::env::var("CARGO_PKG_NAME").ok();

    let target_exe_path = args_iter
        .next()
        .expect("expected path to target executable");

    let args: Vec<(String, Value)> = args_iter.map(|s| Value::parse_pair(&s)).collect();

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
                .unwrap_or_else(|_e| default_config())
        });

    // Parse CLI arguments are key-value pairs
    for (k, v) in args {
        match k.as_str() {
            "boot-type" | "boot_type" => {
                let ty: BootType =
                    serde_plain::from_str(&v.as_string().expect("boot_type expects a string"))
                        .expect("invalid boot_type");
                data.image_runner.boot_type = ty;
            }
            "limine-branch" | "limine_branch" => {
                data.image_runner.limine_branch =
                    v.as_string().expect("limine_branch expects a string");
            }
            "config-file" | "config_file" => {
                data.image_runner.config_file =
                    v.as_string().expect("config_file expects a string");
            }
            var if data.image_runner.vars.contains_key(var) => {
                data.image_runner.vars.insert(
                    var.to_string(),
                    v.as_string().expect("variables should be strings"),
                );
            }
            other => panic!(
                "{} is not a valid config value, arguments should be in the form key=value",
                other
            ),
        }
    }

    let mut parse_ctx = ParseCtx::new(
        data.image_runner,
        PathBuf::from(target_exe_path.as_str()),
        PathBuf::from(root_dir),
    );

    parse_ctx.prepare_bootloader();
    parse_ctx.prepare_iso();
    parse_ctx.run();
}
