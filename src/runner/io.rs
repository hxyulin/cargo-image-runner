//! I/O handler trait and built-in implementations for capturing and streaming
//! serial output from running QEMU instances.
//!
//! The [`IoHandler`] trait enables:
//! - **Real-time output streaming** — process serial output as it arrives
//! - **Programmatic/reactive input** — send serial input in response to output patterns
//! - **Post-run capture** — full output available after run for test assertions
//!
//! # Built-in Handlers
//!
//! - [`CaptureHandler`] — accumulates all serial + stderr bytes, returns them via `finish()`
//! - [`TeeHandler`] — captures AND echoes to real terminal
//! - [`PatternResponder`] — matches string patterns in serial output and sends responses

/// Actions a handler can return to control the runner.
#[derive(Debug)]
pub enum IoAction {
    /// Continue running normally.
    Continue,
    /// Send the given bytes to the QEMU serial input (stdin).
    SendInput(Vec<u8>),
    /// Shut down the QEMU process.
    Shutdown,
}

/// Data captured during a run, returned by [`IoHandler::finish()`].
#[derive(Debug, Clone)]
pub struct CapturedIo {
    /// Captured serial output bytes.
    pub serial: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
}

/// Trait for handling I/O from a running QEMU instance.
///
/// Implementors receive callbacks for serial output, stderr, process start/exit,
/// and can return [`IoAction`]s to send input or shut down the process.
pub trait IoHandler: Send {
    /// Called when serial output bytes arrive. Return an [`IoAction`].
    fn on_output(&mut self, data: &[u8]) -> IoAction {
        let _ = data;
        IoAction::Continue
    }

    /// Called when QEMU stderr produces output.
    fn on_stderr(&mut self, data: &[u8]) {
        let _ = data;
    }

    /// Called when QEMU exits.
    fn on_exit(&mut self, exit_code: i32, timed_out: bool) {
        let _ = (exit_code, timed_out);
    }

    /// Called before QEMU starts with the command being executed.
    fn on_start(&mut self, command: &std::process::Command) {
        let _ = command;
    }

    /// Called after run completes to extract captured data.
    fn finish(self: Box<Self>) -> Option<CapturedIo> {
        None
    }
}

/// Handler that accumulates all serial and stderr bytes for post-run inspection.
///
/// # Example
///
/// ```no_run
/// use cargo_image_runner::runner::io::CaptureHandler;
///
/// let handler = CaptureHandler::new();
/// // Pass to builder via .io_handler(handler)
/// // After run, CapturedIo will contain all serial/stderr output.
/// ```
#[derive(Debug, Default)]
pub struct CaptureHandler {
    serial: Vec<u8>,
    stderr: Vec<u8>,
}

impl CaptureHandler {
    /// Create a new capture handler.
    pub fn new() -> Self {
        Self::default()
    }
}

impl IoHandler for CaptureHandler {
    fn on_output(&mut self, data: &[u8]) -> IoAction {
        self.serial.extend_from_slice(data);
        IoAction::Continue
    }

    fn on_stderr(&mut self, data: &[u8]) {
        self.stderr.extend_from_slice(data);
    }

    fn finish(self: Box<Self>) -> Option<CapturedIo> {
        Some(CapturedIo {
            serial: self.serial,
            stderr: self.stderr,
        })
    }
}

/// Handler that captures output AND echoes it to the real terminal.
///
/// Wraps a [`CaptureHandler`] internally so all data is available via `finish()`.
#[derive(Debug, Default)]
pub struct TeeHandler {
    capture: CaptureHandler,
}

impl TeeHandler {
    /// Create a new tee handler.
    pub fn new() -> Self {
        Self::default()
    }
}

impl IoHandler for TeeHandler {
    fn on_output(&mut self, data: &[u8]) -> IoAction {
        use std::io::Write;
        let _ = std::io::stdout().write_all(data);
        self.capture.on_output(data)
    }

    fn on_stderr(&mut self, data: &[u8]) {
        use std::io::Write;
        let _ = std::io::stderr().write_all(data);
        self.capture.on_stderr(data);
    }

    fn finish(self: Box<Self>) -> Option<CapturedIo> {
        Box::new(self.capture).finish()
    }
}

/// A pattern/response pair for [`PatternResponder`].
#[derive(Debug, Clone)]
struct PatternRule {
    pattern: String,
    response: Vec<u8>,
}

/// Handler that watches serial output for string patterns and sends responses.
///
/// Also captures all output for post-run inspection.
///
/// # Example
///
/// ```no_run
/// use cargo_image_runner::runner::io::PatternResponder;
///
/// let handler = PatternResponder::new()
///     .on_pattern("login:", b"root\n")
///     .on_pattern("$ ", b"run-tests\n");
/// ```
#[derive(Debug, Default)]
pub struct PatternResponder {
    rules: Vec<PatternRule>,
    /// Rolling buffer of recent serial output for pattern matching.
    buffer: Vec<u8>,
    capture: CaptureHandler,
}

impl PatternResponder {
    /// Create a new pattern responder with no rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pattern/response rule. When `pattern` is found in serial output,
    /// `response` bytes will be sent to QEMU stdin.
    pub fn on_pattern(mut self, pattern: &str, response: &[u8]) -> Self {
        self.rules.push(PatternRule {
            pattern: pattern.to_string(),
            response: response.to_vec(),
        });
        self
    }
}

impl IoHandler for PatternResponder {
    fn on_output(&mut self, data: &[u8]) -> IoAction {
        self.capture.on_output(data);
        self.buffer.extend_from_slice(data);

        // Keep buffer size bounded — retain enough for longest pattern match
        let max_pattern_len = self.rules.iter().map(|r| r.pattern.len()).max().unwrap_or(0);
        let max_buf = max_pattern_len.max(4096);
        if self.buffer.len() > max_buf * 2 {
            let drain = self.buffer.len() - max_buf;
            self.buffer.drain(..drain);
        }

        // Check patterns against buffer
        let buf_str = String::from_utf8_lossy(&self.buffer);
        for rule in &self.rules {
            if buf_str.contains(&rule.pattern) {
                // Clear buffer past the match to avoid re-triggering
                self.buffer.clear();
                return IoAction::SendInput(rule.response.clone());
            }
        }

        IoAction::Continue
    }

    fn on_stderr(&mut self, data: &[u8]) {
        self.capture.on_stderr(data);
    }

    fn finish(self: Box<Self>) -> Option<CapturedIo> {
        Box::new(self.capture).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_handler_accumulates() {
        let mut handler = CaptureHandler::new();
        handler.on_output(b"hello ");
        handler.on_output(b"world");
        handler.on_stderr(b"err1");
        handler.on_stderr(b"err2");

        let captured = Box::new(handler).finish().unwrap();
        assert_eq!(captured.serial, b"hello world");
        assert_eq!(captured.stderr, b"err1err2");
    }

    #[test]
    fn test_capture_handler_empty() {
        let handler = CaptureHandler::new();
        let captured = Box::new(handler).finish().unwrap();
        assert!(captured.serial.is_empty());
        assert!(captured.stderr.is_empty());
    }

    #[test]
    fn test_tee_handler_captures() {
        let mut handler = TeeHandler::new();
        handler.on_output(b"data");
        handler.on_stderr(b"err");

        let captured = Box::new(handler).finish().unwrap();
        assert_eq!(captured.serial, b"data");
        assert_eq!(captured.stderr, b"err");
    }

    #[test]
    fn test_pattern_responder_matches() {
        let mut handler = PatternResponder::new()
            .on_pattern("login:", b"root\n")
            .on_pattern("$ ", b"ls\n");

        // No match yet
        let action = handler.on_output(b"booting...\n");
        assert!(matches!(action, IoAction::Continue));

        // Match login:
        let action = handler.on_output(b"login:");
        match action {
            IoAction::SendInput(data) => assert_eq!(data, b"root\n"),
            other => panic!("expected SendInput, got {:?}", other),
        }

        // Match shell prompt
        let action = handler.on_output(b"root@host:~$ ");
        match action {
            IoAction::SendInput(data) => assert_eq!(data, b"ls\n"),
            other => panic!("expected SendInput, got {:?}", other),
        }
    }

    #[test]
    fn test_pattern_responder_captures() {
        let mut handler = PatternResponder::new().on_pattern("x", b"y");
        handler.on_output(b"abc");
        handler.on_stderr(b"err");

        let captured = Box::new(handler).finish().unwrap();
        assert_eq!(captured.serial, b"abc");
        assert_eq!(captured.stderr, b"err");
    }

    #[test]
    fn test_pattern_responder_no_rules() {
        let mut handler = PatternResponder::new();
        let action = handler.on_output(b"anything");
        assert!(matches!(action, IoAction::Continue));
    }

    #[test]
    fn test_default_io_handler_noop() {
        struct Noop;
        impl IoHandler for Noop {}

        let mut handler = Noop;
        let action = handler.on_output(b"data");
        assert!(matches!(action, IoAction::Continue));
        handler.on_stderr(b"err");
        handler.on_exit(0, false);
        assert!(Box::new(handler).finish().is_none());
    }
}
