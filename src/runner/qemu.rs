use super::{RunResult, Runner};
use crate::core::context::Context;
use crate::core::error::{Error, Result};
use crate::firmware::OvmfFirmware;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

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

        // Add env var args (CARGO_IMAGE_RUNNER_QEMU_ARGS)
        for arg in &ctx.env_extra_args {
            cmd.arg(arg);
        }

        // Add CLI passthrough args — only in non-test mode.
        // In test mode, these are forwarded via {{ARGS}} to kernel cmdline instead.
        if !ctx.is_test {
            for arg in &ctx.cli_extra_args {
                cmd.arg(arg);
            }
        }

        // Run QEMU
        if ctx.config.verbose {
            println!("Executing: {:?}", cmd);
        }

        if ctx.is_test {
            // Test mode: inherit stdout/stderr so serial output is visible
            // in real-time, enforce timeout, check exit code.
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());

            let child = cmd.spawn().map_err(|e| {
                Error::runner(format!(
                    "failed to execute {}: {}",
                    qemu_config.binary, e
                ))
            })?;

            // Set up timeout watchdog
            let timed_out = Arc::new(AtomicBool::new(false));
            let _timeout_handle = if let Some(timeout_secs) = ctx.config.test.timeout {
                let flag = timed_out.clone();
                let child_id = child.id();
                Some(std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(timeout_secs));
                    if !flag.swap(true, Ordering::SeqCst) {
                        #[cfg(unix)]
                        {
                            unsafe {
                                libc::kill(child_id as i32, libc::SIGKILL);
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            let _ = child_id;
                        }
                    }
                }))
            } else {
                None
            };

            let status = child.wait_with_output().map_err(|e| {
                Error::runner(format!("failed to wait for {}: {}", qemu_config.binary, e))
            })?;

            // Signal the timeout thread that we're done (prevent spurious kill).
            // Don't join — it would block until the full timeout elapses.
            // The thread checks the flag and returns without killing.
            let was_timed_out = timed_out.swap(true, Ordering::SeqCst);

            let exit_code = status.status.code().unwrap_or(-1);
            let success = if let Some(success_code) = ctx.test_success_exit_code() {
                exit_code == success_code
            } else {
                status.status.success()
            };

            let mut result = RunResult::new(exit_code, success);
            if was_timed_out {
                result = result.with_timeout();
            }
            Ok(result)
        } else {
            // Normal mode: inherit stdio
            cmd.stdin(Stdio::inherit());
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());

            let status = cmd.status().map_err(|e| {
                Error::runner(format!(
                    "failed to execute {}: {}",
                    qemu_config.binary, e
                ))
            })?;

            let exit_code = status.code().unwrap_or(-1);
            Ok(RunResult::new(exit_code, status.success()))
        }
    }

    fn is_available(&self) -> bool {
        Self::check_available()
    }

    fn name(&self) -> &str {
        "QEMU"
    }
}
