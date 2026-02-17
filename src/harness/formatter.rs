//! Terminal output formatting for test harness results.

use crate::config::ShowOutput;
use crate::runner::CapturedOutput;

use super::{TestCaseStatus, TestOutput};

/// Formats and prints test harness results to the terminal.
pub struct ResultFormatter {
    show_output: ShowOutput,
}

impl ResultFormatter {
    /// Create a new formatter with the given output display policy.
    pub fn new(show_output: ShowOutput) -> Self {
        Self { show_output }
    }

    /// Print the full test report: individual results, captured output, and summary.
    pub fn report(&self, output: &TestOutput, captured: Option<&CapturedOutput>) {
        self.print_cases(output);
        self.print_captured_output(output, captured);
        self.print_summary(output);
    }

    /// Print individual test case results.
    fn print_cases(&self, output: &TestOutput) {
        for case in &output.cases {
            let marker = match case.status {
                TestCaseStatus::Passed => "PASS",
                TestCaseStatus::Failed => "FAIL",
            };
            println!("[{marker}] {}", case.name);
        }
    }

    /// Print captured output based on the show_output policy.
    fn print_captured_output(&self, output: &TestOutput, captured: Option<&CapturedOutput>) {
        let captured = match captured {
            Some(c) => c,
            None => return,
        };

        let should_show = match self.show_output {
            ShowOutput::Always => true,
            ShowOutput::Never => false,
            ShowOutput::OnFailure => !output.overall_success,
        };

        if !should_show {
            return;
        }

        if !captured.stdout.is_empty() {
            println!("\n--- captured stdout ---");
            print!("{}", captured.stdout);
            if !captured.stdout.ends_with('\n') {
                println!();
            }
            println!("--- end stdout ---");
        }

        if !captured.stderr.is_empty() {
            println!("\n--- captured stderr ---");
            print!("{}", captured.stderr);
            if !captured.stderr.ends_with('\n') {
                println!();
            }
            println!("--- end stderr ---");
        }
    }

    /// Print the test summary line.
    fn print_summary(&self, output: &TestOutput) {
        let status = if output.overall_success {
            "ok"
        } else {
            "FAILED"
        };

        print!(
            "\ntest result: {status}. {} passed, {} failed",
            output.passed, output.failed
        );

        if output.timed_out {
            print!(" (timed out)");
        }

        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::parser::{TestCaseResult, TestCaseStatus};

    #[test]
    fn test_formatter_creation() {
        let fmt = ResultFormatter::new(ShowOutput::OnFailure);
        assert_eq!(fmt.show_output, ShowOutput::OnFailure);
    }

    #[test]
    fn test_report_does_not_panic() {
        let fmt = ResultFormatter::new(ShowOutput::Always);
        let output = TestOutput {
            cases: vec![
                TestCaseResult {
                    name: "test_a".to_string(),
                    status: TestCaseStatus::Passed,
                },
                TestCaseResult {
                    name: "test_b".to_string(),
                    status: TestCaseStatus::Failed,
                },
            ],
            passed: 1,
            failed: 1,
            timed_out: false,
            overall_success: false,
        };
        let captured = CapturedOutput {
            stdout: "some output\n".to_string(),
            stderr: "some error\n".to_string(),
        };
        // Just verify it doesn't panic
        fmt.report(&output, Some(&captured));
    }

    #[test]
    fn test_report_no_captured_output() {
        let fmt = ResultFormatter::new(ShowOutput::Always);
        let output = TestOutput {
            cases: vec![],
            passed: 0,
            failed: 0,
            timed_out: false,
            overall_success: true,
        };
        // Should not panic with None captured output
        fmt.report(&output, None);
    }

    #[test]
    fn test_report_on_failure_hides_output_on_success() {
        let fmt = ResultFormatter::new(ShowOutput::OnFailure);
        let output = TestOutput {
            cases: vec![TestCaseResult {
                name: "test_ok".to_string(),
                status: TestCaseStatus::Passed,
            }],
            passed: 1,
            failed: 0,
            timed_out: false,
            overall_success: true,
        };
        let captured = CapturedOutput {
            stdout: "should not be printed\n".to_string(),
            stderr: String::new(),
        };
        // Should not panic; on_failure mode with success means output is suppressed
        fmt.report(&output, Some(&captured));
    }
}
