use crate::core::error::Result;
use std::collections::HashMap;

/// Template processor for substituting variables in configuration files.
pub struct TemplateProcessor;

impl TemplateProcessor {
    /// Process template variables in content.
    ///
    /// Supports both {{VAR}} and $VAR syntax.
    pub fn process(content: &str, vars: &HashMap<String, String>) -> Result<String> {
        let mut result = content.to_string();

        // Process {{VAR}} syntax
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Process $VAR syntax
        // This is a simple implementation - could be enhanced to handle ${VAR} etc.
        for (key, value) in vars {
            let placeholder = format!("${}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_double_brace() {
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "test".to_string());
        vars.insert("VALUE".to_string(), "123".to_string());

        let content = "Hello {{NAME}}, value is {{VALUE}}";
        let result = TemplateProcessor::process(content, &vars).unwrap();
        assert_eq!(result, "Hello test, value is 123");
    }

    #[test]
    fn test_template_dollar() {
        let mut vars = HashMap::new();
        vars.insert("VAR".to_string(), "substituted".to_string());

        let content = "This is $VAR";
        let result = TemplateProcessor::process(content, &vars).unwrap();
        assert_eq!(result, "This is substituted");
    }

    #[test]
    fn test_template_mixed() {
        let mut vars = HashMap::new();
        vars.insert("A".to_string(), "alpha".to_string());
        vars.insert("B".to_string(), "beta".to_string());

        let content = "{{A}} and $B";
        let result = TemplateProcessor::process(content, &vars).unwrap();
        assert_eq!(result, "alpha and beta");
    }

    #[test]
    fn test_template_no_vars() {
        let vars = HashMap::new();
        let content = "Hello, world!";
        let result = TemplateProcessor::process(content, &vars).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_template_empty_content() {
        let vars = HashMap::new();
        let result = TemplateProcessor::process("", &vars).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_template_unknown_var_preserved() {
        let mut vars = HashMap::new();
        vars.insert("KNOWN".to_string(), "value".to_string());

        let content = "{{KNOWN}} and {{UNKNOWN}}";
        let result = TemplateProcessor::process(content, &vars).unwrap();
        assert_eq!(result, "value and {{UNKNOWN}}");
    }

    #[test]
    fn test_template_repeated_var() {
        let mut vars = HashMap::new();
        vars.insert("X".to_string(), "42".to_string());

        let content = "{{X}} + {{X}} = 2*{{X}}";
        let result = TemplateProcessor::process(content, &vars).unwrap();
        assert_eq!(result, "42 + 42 = 2*42");
    }

    #[test]
    fn test_template_multiline() {
        let mut vars = HashMap::new();
        vars.insert("TIMEOUT".to_string(), "5".to_string());
        vars.insert("EXECUTABLE_NAME".to_string(), "kernel.elf".to_string());
        vars.insert("CMDLINE".to_string(), "quiet".to_string());

        let content = "timeout: {{TIMEOUT}}\n\n/My Kernel\n    protocol: limine\n    kernel_path: boot():/boot/{{EXECUTABLE_NAME}}\n    cmdline: {{CMDLINE}}";
        let result = TemplateProcessor::process(content, &vars).unwrap();
        assert_eq!(
            result,
            "timeout: 5\n\n/My Kernel\n    protocol: limine\n    kernel_path: boot():/boot/kernel.elf\n    cmdline: quiet"
        );
    }
}
