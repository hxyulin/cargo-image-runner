use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Value;

/// An enum representing the boot type to use
#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum BootType {
    #[default]
    #[serde(rename = "bios")]
    Bios,
    #[serde(rename = "uefi")]
    Uefi,
}

const fn def_test_success_exit_code() -> u32 {
    33
}

#[derive(Debug, Deserialize)]
pub struct ImageRunnerConfig {
    #[serde(rename = "config-file")]
    pub config_file: String,
    #[serde(default)]
    #[serde(rename = "extra-files")]
    pub extra_files: Vec<String>,
    #[serde(rename = "limine-branch")]
    pub limine_branch: String,
    #[serde(rename = "run-command")]
    pub run_command: Vec<String>,
    #[serde(rename = "test-args")]
    #[serde(default)]
    pub test_args: Vec<String>,
    #[serde(rename = "run-args")]
    #[serde(default)]
    pub run_args: Vec<String>,
    #[serde(rename = "test-success-exit-code")]
    #[serde(default = "def_test_success_exit_code")]
    pub test_success_exit_code: u32,
    #[serde(rename = "boot-type")]
    #[serde(default)]
    pub boot_type: BootType,
    /// The kernel command line to use
    #[serde(default)]
    pub cmdline: String,
    #[serde(default)]
    pub vars: HashMap<String, String>,
}

pub fn default_config() -> PackageMetadata {
    PackageMetadata {
        image_runner: ImageRunnerConfig {
            config_file: "limine.conf".to_string(),
            extra_files: vec![],
            limine_branch: "v8.x-binary".to_string(),
            run_command: vec!["qemu-system-x86_64".to_string(), "-cdrom".to_string(), "{}".to_string()],
            test_args: vec![],
            run_args: vec![],
            test_success_exit_code: 33,
            boot_type: BootType::Bios,
            cmdline: "".to_string(),
            vars: HashMap::new(),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct PackageMetadata {
    #[serde(rename = "image-runner")]
    pub image_runner: ImageRunnerConfig,
}
