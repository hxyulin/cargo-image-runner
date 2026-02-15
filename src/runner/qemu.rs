use super::{RunResult, Runner};
use crate::core::context::Context;
use crate::core::error::{Error, Result};
use crate::firmware::OvmfFirmware;
use std::path::Path;
use std::process::{Command, Stdio};

/// QEMU runner for executing bootable images.
pub struct QemuRunner;

impl QemuRunner {
    /// Create a new QEMU runner.
    pub fn new() -> Self {
        Self
    }

    /// Check if QEMU is available.
    fn check_available() -> bool {
        Command::new("qemu-system-x86_64")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }
}

impl Default for QemuRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for QemuRunner {
    fn run(&self, ctx: &Context, image_path: &Path) -> Result<RunResult> {
        let qemu_config = &ctx.config.runner.qemu;

        let mut cmd = Command::new(&qemu_config.binary);

        // Basic QEMU args
        cmd.arg("-machine").arg(&qemu_config.machine);
        cmd.arg("-m").arg(qemu_config.memory.to_string());

        // CPU cores
        if qemu_config.cores > 1 {
            cmd.arg("-smp").arg(qemu_config.cores.to_string());
        }

        // Serial output
        cmd.arg("-serial").arg("mon:stdio");

        // KVM acceleration if enabled and available
        #[cfg(target_os = "linux")]
        if qemu_config.kvm {
            cmd.arg("-enable-kvm");
        }

        // Handle UEFI boot
        if ctx.config.boot.boot_type.needs_uefi() {
            #[cfg(feature = "uefi")]
            {
                // Fetch OVMF firmware
                let ovmf = OvmfFirmware::new(ctx.cache_dir.join("ovmf"));
                let ovmf_files = ovmf.fetch()?;

                // Add OVMF code and vars
                cmd.arg("-drive").arg(format!(
                    "if=pflash,format=raw,readonly=on,file={}",
                    ovmf_files.code().display()
                ));
                cmd.arg("-drive").arg(format!(
                    "if=pflash,format=raw,file={}",
                    ovmf_files.vars().display()
                ));
            }

            #[cfg(not(feature = "uefi"))]
            {
                return Err(Error::feature_not_enabled(
                    "uefi (required for UEFI boot)",
                ));
            }
        }

        // Attach the image
        // For directory output, use fat:rw:
        // For ISO/FAT images, use -cdrom or -drive
        if image_path.is_dir() {
            cmd.arg("-drive").arg(format!(
                "format=raw,file=fat:rw:{}",
                image_path.display()
            ));
        } else if image_path.extension().and_then(|s| s.to_str()) == Some("iso") {
            cmd.arg("-cdrom").arg(image_path);
        } else {
            cmd.arg("-drive")
                .arg(format!("format=raw,file={}", image_path.display()));
        }

        // Add extra arguments from config
        let extra_args = ctx.get_extra_args();
        for arg in extra_args {
            cmd.arg(arg);
        }

        // Add any additional QEMU arguments from config
        for arg in &qemu_config.extra_args {
            cmd.arg(arg);
        }

        // Set up stdio
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        // Run QEMU
        println!("Executing: {:?}", cmd);
        let status = cmd.status().map_err(|e| {
            Error::runner(format!(
                "failed to execute {}: {}",
                qemu_config.binary, e
            ))
        })?;

        let exit_code = status.code().unwrap_or(-1);

        // Check for test success code
        if ctx.is_test {
            if let Some(success_code) = ctx.test_success_exit_code() {
                return Ok(RunResult::new(exit_code, exit_code == success_code));
            }
        }

        // Otherwise, success is exit code 0
        Ok(RunResult::new(exit_code, status.success()))
    }

    fn is_available(&self) -> bool {
        Self::check_available()
    }

    fn name(&self) -> &str {
        "QEMU"
    }
}
