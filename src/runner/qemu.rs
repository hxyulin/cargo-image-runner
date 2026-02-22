use super::io::{IoAction, IoHandler};
use super::{RunResult, Runner};
use crate::config::SerialMode;
use crate::core::context::Context;
use crate::core::error::{Error, Result};
use crate::firmware::OvmfFirmware;
use std::io::Read as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
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

    /// Build the base QEMU command with machine, memory, cores, KVM, UEFI,
    /// image, and extra arguments — but NOT serial or stdio config.
    fn build_command(&self, ctx: &Context, image_path: &Path) -> Result<Command> {
        let qemu_config = &ctx.config.runner.qemu;

        let mut cmd = Command::new(&qemu_config.binary);

        // Basic QEMU args
        cmd.arg("-machine").arg(&qemu_config.machine);
        cmd.arg("-m").arg(qemu_config.memory.to_string());

        // CPU cores
        if qemu_config.cores > 1 {
            cmd.arg("-smp").arg(qemu_config.cores.to_string());
        }

        // KVM acceleration if enabled and available
        #[cfg(target_os = "linux")]
        if qemu_config.kvm {
            cmd.arg("-enable-kvm");
        }

        // Handle UEFI boot
        if ctx.config.boot.boot_type.needs_uefi() {
            #[cfg(feature = "uefi")]
            {
                let ovmf = OvmfFirmware::new(ctx.cache_dir.join("ovmf"));
                let ovmf_files = ovmf.fetch()?;

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

        // Add extra arguments from config (test or run mode)
        for arg in ctx.get_extra_args() {
            cmd.arg(arg);
        }

        // Add extra QEMU arguments from config
        for arg in &qemu_config.extra_args {
            cmd.arg(arg);
        }

        // Add env var args (CARGO_IMAGE_RUNNER_QEMU_ARGS)
        for arg in &ctx.env_extra_args {
            cmd.arg(arg);
        }

        // Add CLI passthrough args — only in non-test mode.
        if !ctx.is_test {
            for arg in &ctx.cli_extra_args {
                cmd.arg(arg);
            }
        }

        Ok(cmd)
    }

    /// Apply serial and monitor flags to a command based on SerialConfig.
    fn apply_serial_config(cmd: &mut Command, mode: SerialMode, separate_monitor: Option<bool>) {
        let serial_arg = match mode {
            SerialMode::MonStdio => "mon:stdio",
            SerialMode::Stdio => "stdio",
            SerialMode::None => "none",
        };
        cmd.arg("-serial").arg(serial_arg);

        // If separate_monitor is explicitly requested and mode is not mon:stdio
        // (which already includes the monitor), add a separate monitor.
        if separate_monitor == Some(true) && mode != SerialMode::MonStdio {
            cmd.arg("-monitor").arg("none");
        }
    }

    /// Set up timeout watchdog thread. Returns the timed_out flag.
    fn setup_timeout(
        timeout_secs: Option<u64>,
        child_id: u32,
    ) -> (Arc<AtomicBool>, Option<std::thread::JoinHandle<()>>) {
        let timed_out = Arc::new(AtomicBool::new(false));
        let handle = if let Some(secs) = timeout_secs {
            let flag = timed_out.clone();
            Some(std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(secs));
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
        (timed_out, handle)
    }
}

impl Default for QemuRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal events sent from reader threads to the main I/O loop.
enum IoEvent {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    StdoutClosed,
    StderrClosed,
}

impl Runner for QemuRunner {
    fn run(&self, ctx: &Context, image_path: &Path) -> Result<RunResult> {
        let qemu_config = &ctx.config.runner.qemu;
        let mut cmd = self.build_command(ctx, image_path)?;

        // Apply serial config from settings
        Self::apply_serial_config(
            &mut cmd,
            qemu_config.serial.mode,
            qemu_config.serial.separate_monitor,
        );

        if ctx.config.verbose {
            println!("Executing: {:?}", cmd);
        }

        if ctx.is_test {
            // Test mode: inherit stdout/stderr, enforce timeout, check exit code.
            cmd.stdin(Stdio::null());
            cmd.stdout(Stdio::inherit());
            cmd.stderr(Stdio::inherit());

            let child = cmd.spawn().map_err(|e| {
                Error::runner(format!(
                    "failed to execute {}: {}",
                    qemu_config.binary, e
                ))
            })?;

            let (timed_out, _timeout_handle) =
                Self::setup_timeout(ctx.config.test.timeout, child.id());

            let status = child.wait_with_output().map_err(|e| {
                Error::runner(format!("failed to wait for {}: {}", qemu_config.binary, e))
            })?;

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

    fn run_with_io(
        &self,
        ctx: &Context,
        image_path: &Path,
        handler: &mut dyn IoHandler,
    ) -> Result<RunResult> {
        let qemu_config = &ctx.config.runner.qemu;
        let mut cmd = self.build_command(ctx, image_path)?;

        // When using an I/O handler, force serial to stdio and disable the monitor
        // so stdout carries only serial data.
        cmd.arg("-serial").arg("stdio");
        cmd.arg("-monitor").arg("none");

        // Pipe all stdio for programmatic access
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if ctx.config.verbose {
            println!("Executing (with I/O handler): {:?}", cmd);
        }

        handler.on_start(&cmd);

        let mut child = cmd.spawn().map_err(|e| {
            Error::runner(format!(
                "failed to execute {}: {}",
                qemu_config.binary, e
            ))
        })?;

        let child_id = child.id();

        // Take ownership of piped streams
        let mut child_stdin = child.stdin.take();
        let child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::runner("failed to capture QEMU stdout"))?;
        let child_stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::runner("failed to capture QEMU stderr"))?;

        // Set up I/O event channel
        let (tx, rx) = mpsc::channel::<IoEvent>();

        // Spawn stdout reader thread
        let tx_stdout = tx.clone();
        let stdout_thread = std::thread::spawn(move || {
            let mut reader = child_stdout;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx_stdout.send(IoEvent::StdoutClosed);
                        break;
                    }
                    Ok(n) => {
                        if tx_stdout.send(IoEvent::Stdout(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = tx_stdout.send(IoEvent::StdoutClosed);
                        break;
                    }
                }
            }
        });

        // Spawn stderr reader thread
        let tx_stderr = tx;
        let stderr_thread = std::thread::spawn(move || {
            let mut reader = child_stderr;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        let _ = tx_stderr.send(IoEvent::StderrClosed);
                        break;
                    }
                    Ok(n) => {
                        if tx_stderr.send(IoEvent::Stderr(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = tx_stderr.send(IoEvent::StderrClosed);
                        break;
                    }
                }
            }
        });

        // Set up timeout watchdog
        let (timed_out, _timeout_handle) =
            Self::setup_timeout(ctx.config.test.timeout, child_id);

        // Main I/O event loop
        let mut stdout_closed = false;
        let mut stderr_closed = false;

        while !stdout_closed || !stderr_closed {
            let event = match rx.recv() {
                Ok(event) => event,
                Err(_) => break,
            };

            match event {
                IoEvent::Stdout(data) => {
                    let action = handler.on_output(&data);
                    match action {
                        IoAction::Continue => {}
                        IoAction::SendInput(input) => {
                            if let Some(ref mut stdin) = child_stdin {
                                let _ = stdin.write_all(&input);
                                let _ = stdin.flush();
                            }
                        }
                        IoAction::Shutdown => {
                            // Kill the child process
                            let _ = child.kill();
                            break;
                        }
                    }
                }
                IoEvent::Stderr(data) => {
                    handler.on_stderr(&data);
                }
                IoEvent::StdoutClosed => {
                    stdout_closed = true;
                }
                IoEvent::StderrClosed => {
                    stderr_closed = true;
                }
            }
        }

        // Drop stdin to unblock child if it's waiting for input
        drop(child_stdin);

        // Wait for child to exit
        let status = child.wait().map_err(|e| {
            Error::runner(format!("failed to wait for {}: {}", qemu_config.binary, e))
        })?;

        // Join reader threads
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        // Signal the timeout thread that we're done
        let was_timed_out = timed_out.swap(true, Ordering::SeqCst);

        let exit_code = status.code().unwrap_or(-1);
        let success = if let Some(success_code) = ctx.test_success_exit_code() {
            exit_code == success_code
        } else {
            status.success()
        };

        handler.on_exit(exit_code, was_timed_out);

        let mut result = RunResult::new(exit_code, success);
        if was_timed_out {
            result = result.with_timeout();
        }

        Ok(result)
    }

    fn is_available(&self) -> bool {
        Self::check_available()
    }

    fn name(&self) -> &str {
        "QEMU"
    }
}
