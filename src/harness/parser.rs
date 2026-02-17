//! Regex-based output parser for extracting sub-test results from serial output.

use regex::Regex;

use crate::config::HarnessConfig;
use crate::core::error::{Error, Result};

/// Parses runner output to extract individual test case results.
pub struct OutputParser {
    pass_regex: Regex,
    fail_regex: Regex,
}

/// Result of a single test case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseResult {
    pub name: String,
    pub status: TestCaseStatus,
}

/// Status of a single test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCaseStatus {
    Passed,
    Failed,
}

impl OutputParser {
    /// Create a new parser from harness configuration.
    pub fn new(config: &HarnessConfig) -> Result<Self> {
        let pass_regex = Regex::new(&config.pass_pattern).map_err(|e| {
            Error::test_harness(format!("invalid pass pattern '{}': {}", config.pass_pattern, e))
        })?;
        let fail_regex = Regex::new(&config.fail_pattern).map_err(|e| {
            Error::test_harness(format!("invalid fail pattern '{}': {}", config.fail_pattern, e))
        })?;

        Ok(Self {
            pass_regex,
            fail_regex,
        })
    }

    /// Parse output text and extract test case results.
    pub fn parse(&self, output: &str) -> Vec<TestCaseResult> {
        let mut results = Vec::new();

        for line in output.lines() {
            if let Some(caps) = self.pass_regex.captures(line) {
                if let Some(name) = caps.get(1) {
                    results.push(TestCaseResult {
                        name: name.as_str().trim().to_string(),
                        status: TestCaseStatus::Passed,
                    });
                }
            } else if let Some(caps) = self.fail_regex.captures(line) {
                if let Some(name) = caps.get(1) {
                    results.push(TestCaseResult {
                        name: name.as_str().trim().to_string(),
                        status: TestCaseStatus::Failed,
                    });
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HarnessConfig;

    fn default_parser() -> OutputParser {
        OutputParser::new(&HarnessConfig::default()).unwrap()
    }

    #[test]
    fn test_parse_pass_lines() {
        let parser = default_parser();
        let output = "[PASS] test_basic_allocation\n[OK] test_page_fault_handler\n[PASSED] test_heap\n";
        let results = parser.parse(output);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "test_basic_allocation");
        assert_eq!(results[0].status, TestCaseStatus::Passed);
        assert_eq!(results[1].name, "test_page_fault_handler");
        assert_eq!(results[1].status, TestCaseStatus::Passed);
        assert_eq!(results[2].name, "test_heap");
        assert_eq!(results[2].status, TestCaseStatus::Passed);
    }

    #[test]
    fn test_parse_fail_lines() {
        let parser = default_parser();
        let output = "[FAIL] test_stack_overflow\n[FAILED] test_double_fault\n[ERROR] test_panic\n";
        let results = parser.parse(output);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "test_stack_overflow");
        assert_eq!(results[0].status, TestCaseStatus::Failed);
        assert_eq!(results[1].name, "test_double_fault");
        assert_eq!(results[1].status, TestCaseStatus::Failed);
        assert_eq!(results[2].name, "test_panic");
        assert_eq!(results[2].status, TestCaseStatus::Failed);
    }

    #[test]
    fn test_parse_mixed_output() {
        let parser = default_parser();
        let output = "\
Booting kernel...
Running tests:
[PASS] test_basic_allocation
[PASS] test_page_fault_handler
[FAIL] test_stack_overflow
Some other output
[OK] test_heap_alloc
";
        let results = parser.parse(output);
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].status, TestCaseStatus::Passed);
        assert_eq!(results[1].status, TestCaseStatus::Passed);
        assert_eq!(results[2].status, TestCaseStatus::Failed);
        assert_eq!(results[3].status, TestCaseStatus::Passed);
    }

    #[test]
    fn test_parse_no_matches() {
        let parser = default_parser();
        let output = "Booting kernel...\nHello World!\nShutting down.\n";
        let results = parser.parse(output);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_empty_output() {
        let parser = default_parser();
        let results = parser.parse("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_custom_patterns() {
        let config = HarnessConfig {
            pass_pattern: r"PASSED:\s+(.+)".to_string(),
            fail_pattern: r"FAILED:\s+(.+)".to_string(),
            ..Default::default()
        };
        let parser = OutputParser::new(&config).unwrap();
        let output = "PASSED: my_test\nFAILED: broken_test\n";
        let results = parser.parse(output);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "my_test");
        assert_eq!(results[0].status, TestCaseStatus::Passed);
        assert_eq!(results[1].name, "broken_test");
        assert_eq!(results[1].status, TestCaseStatus::Failed);
    }

    #[test]
    fn test_invalid_regex_returns_error() {
        let config = HarnessConfig {
            pass_pattern: r"[invalid".to_string(),
            ..Default::default()
        };
        let result = OutputParser::new(&config);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("invalid pass pattern"));
    }
}
