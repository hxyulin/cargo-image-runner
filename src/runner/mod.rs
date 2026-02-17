//! Runner trait and QEMU implementation for executing bootable images.

use crate::core::context::Context;
use crate::core::error::Result;
use std::path::Path;

// Runner implementations
#[cfg(feature = "qemu")]
pub mod qemu;

/// Runner trait for executing images.
pub trait Runner: Send + Sync {
    /// Execute the image.
    ///
    /// Returns information about the run result.
    fn run(&self, ctx: &Context, image_path: &Path) -> Result<RunResult>;

    /// Check if the runner is available on the system.
    fn is_available(&self) -> bool;

    /// Validate runner configuration.
    fn validate(&self, ctx: &Context) -> Result<()> {
        if !self.is_available() {
            return Err(crate::core::error::Error::runner(format!(
                "{} is not available on this system",
                self.name()
            )));
        }

        let _ = ctx;
        Ok(())
    }

    /// Get a human-readable name for this runner.
    fn name(&self) -> &str;
}

/// Result of running an image.
#[derive(Debug)]
pub struct RunResult {
    /// Exit code from the runner.
    pub exit_code: i32,

    /// Whether the run was considered successful.
    pub success: bool,

    /// Captured stdout/stderr output (populated in test mode).
    pub captured_output: Option<CapturedOutput>,

    /// Whether the run was terminated due to a timeout.
    pub timed_out: bool,
}

/// Captured stdout and stderr from a runner execution.
#[derive(Debug, Clone)]
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

impl RunResult {
    /// Create a new run result.
    pub fn new(exit_code: i32, success: bool) -> Self {
        Self {
            exit_code,
            success,
            captured_output: None,
            timed_out: false,
        }
    }

    /// Create a successful result with exit code 0.
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            success: true,
            captured_output: None,
            timed_out: false,
        }
    }

    /// Create a failed result with the given exit code.
    pub fn failed(exit_code: i32) -> Self {
        Self {
            exit_code,
            success: false,
            captured_output: None,
            timed_out: false,
        }
    }

    /// Attach captured output to the result.
    pub fn with_output(mut self, stdout: String, stderr: String) -> Self {
        self.captured_output = Some(CapturedOutput { stdout, stderr });
        self
    }

    /// Mark the result as timed out.
    pub fn with_timeout(mut self) -> Self {
        self.timed_out = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_result_success() {
        let result = RunResult::success();
        assert_eq!(result.exit_code, 0);
        assert!(result.success);
    }

    #[test]
    fn test_run_result_failed() {
        let result = RunResult::failed(1);
        assert_eq!(result.exit_code, 1);
        assert!(!result.success);
    }

    #[test]
    fn test_run_result_custom() {
        let result = RunResult::new(33, true);
        assert_eq!(result.exit_code, 33);
        assert!(result.success);
    }
}
