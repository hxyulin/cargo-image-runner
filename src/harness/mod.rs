//! Test harness for parsing sub-test results from runner output.
//!
//! The harness sits after the runner in the pipeline: `Bootloader → ImageBuilder → Runner → TestHarness`.
//! The runner captures raw output; the harness interprets it using configurable regex patterns.

mod formatter;
mod parser;

pub use formatter::ResultFormatter;
pub use parser::{OutputParser, TestCaseResult, TestCaseStatus};

use crate::config::HarnessConfig;
use crate::core::error::Result;
use crate::runner::{CapturedOutput, RunResult};

/// Aggregated test output after parsing runner results.
#[derive(Debug)]
pub struct TestOutput {
    /// Individual test case results extracted from output.
    pub cases: Vec<TestCaseResult>,
    /// Number of passed tests.
    pub passed: usize,
    /// Number of failed tests.
    pub failed: usize,
    /// Whether the run was terminated due to a timeout.
    pub timed_out: bool,
    /// Overall success: all tests passed and no timeout.
    pub overall_success: bool,
}

/// Test harness that evaluates runner output and reports results.
pub struct TestHarness {
    parser: OutputParser,
    formatter: ResultFormatter,
}

impl TestHarness {
    /// Create a new test harness from configuration.
    pub fn new(config: &HarnessConfig) -> Result<Self> {
        let parser = OutputParser::new(config)?;
        let formatter = ResultFormatter::new(config.show_output);
        Ok(Self { parser, formatter })
    }

    /// Evaluate runner output to extract test results.
    pub fn evaluate(&self, result: &RunResult) -> TestOutput {
        let cases = if let Some(ref captured) = result.captured_output {
            self.parser.parse(&captured.stdout)
        } else {
            Vec::new()
        };

        let passed = cases
            .iter()
            .filter(|c| c.status == TestCaseStatus::Passed)
            .count();
        let failed = cases
            .iter()
            .filter(|c| c.status == TestCaseStatus::Failed)
            .count();

        let timed_out = result.timed_out;

        // Overall success: no failed tests, no timeout, and runner itself reported success
        let overall_success = failed == 0 && !timed_out && result.success;

        TestOutput {
            cases,
            passed,
            failed,
            timed_out,
            overall_success,
        }
    }

    /// Print the test report to the terminal.
    pub fn report(&self, output: &TestOutput, captured: Option<&CapturedOutput>) {
        self.formatter.report(output, captured);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HarnessConfig;
    use crate::runner::RunResult;

    #[test]
    fn test_harness_evaluate_mixed_results() {
        let harness = TestHarness::new(&HarnessConfig::default()).unwrap();
        let result = RunResult::new(33, true).with_output(
            "[PASS] test_a\n[PASS] test_b\n[FAIL] test_c\n".to_string(),
            String::new(),
        );
        let output = harness.evaluate(&result);
        assert_eq!(output.passed, 2);
        assert_eq!(output.failed, 1);
        assert!(!output.overall_success);
        assert!(!output.timed_out);
    }

    #[test]
    fn test_harness_evaluate_all_pass() {
        let harness = TestHarness::new(&HarnessConfig::default()).unwrap();
        let result = RunResult::new(0, true).with_output(
            "[PASS] test_a\n[OK] test_b\n".to_string(),
            String::new(),
        );
        let output = harness.evaluate(&result);
        assert_eq!(output.passed, 2);
        assert_eq!(output.failed, 0);
        assert!(output.overall_success);
    }

    #[test]
    fn test_harness_evaluate_no_output() {
        let harness = TestHarness::new(&HarnessConfig::default()).unwrap();
        let result = RunResult::new(0, true);
        let output = harness.evaluate(&result);
        assert_eq!(output.passed, 0);
        assert_eq!(output.failed, 0);
        assert!(output.overall_success);
    }

    #[test]
    fn test_harness_evaluate_timeout() {
        let harness = TestHarness::new(&HarnessConfig::default()).unwrap();
        let result = RunResult::new(-1, false)
            .with_output("[PASS] test_a\n".to_string(), String::new())
            .with_timeout();
        let output = harness.evaluate(&result);
        assert_eq!(output.passed, 1);
        assert_eq!(output.failed, 0);
        assert!(output.timed_out);
        assert!(!output.overall_success);
    }

    #[test]
    fn test_harness_evaluate_runner_failure_no_test_output() {
        let harness = TestHarness::new(&HarnessConfig::default()).unwrap();
        let result =
            RunResult::new(1, false).with_output(String::new(), "error occurred\n".to_string());
        let output = harness.evaluate(&result);
        assert_eq!(output.passed, 0);
        assert_eq!(output.failed, 0);
        assert!(!output.overall_success);
    }

    #[test]
    fn test_harness_report_does_not_panic() {
        let harness = TestHarness::new(&HarnessConfig::default()).unwrap();
        let result = RunResult::new(0, true).with_output(
            "[PASS] test_a\n[FAIL] test_b\n".to_string(),
            "some stderr\n".to_string(),
        );
        let output = harness.evaluate(&result);
        harness.report(&output, result.captured_output.as_ref());
    }
}
