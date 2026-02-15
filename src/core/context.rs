use crate::config::Config;
use crate::core::error::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// Context object that carries state through the build pipeline.
pub struct Context {
    /// Configuration.
    pub config: Config,

    /// Workspace root directory.
    pub workspace_root: PathBuf,

    /// Target directory for build artifacts.
    pub target_dir: PathBuf,

    /// Path to the executable being run.
    pub executable: PathBuf,

    /// Whether this is a test run.
    pub is_test: bool,

    /// Cache directory for downloaded/generated files.
    pub cache_dir: PathBuf,

    /// Output directory for the current build.
    pub output_dir: PathBuf,

    /// Template variables available for substitution.
    pub template_vars: HashMap<String, String>,
}

impl Context {
    /// Create a new context from configuration and executable path.
    pub fn new(config: Config, workspace_root: PathBuf, executable: PathBuf) -> Result<Self> {
        let target_dir = workspace_root.join("target").join("image-runner");
        let cache_dir = target_dir.join("cache");
        let output_dir = target_dir.join("output");

        // Ensure directories exist
        std::fs::create_dir_all(&cache_dir)?;
        std::fs::create_dir_all(&output_dir)?;

        let mut ctx = Self {
            config,
            workspace_root: workspace_root.clone(),
            target_dir,
            executable: executable.clone(),
            is_test: false,
            cache_dir,
            output_dir,
            template_vars: HashMap::new(),
        };

        // Detect if this is a test run
        ctx.detect_test();

        // Initialize template variables
        ctx.init_template_vars();

        Ok(ctx)
    }

    /// Detect if the executable is a test binary.
    ///
    /// Tests are detected by checking if the executable name ends with a hash
    /// (Cargo appends a hash suffix to test binaries).
    pub fn detect_test(&mut self) {
        if let Some(file_name) = self.executable.file_name().and_then(|n| n.to_str()) {
            // Test binaries typically have a hash suffix like "mytest-a1b2c3d4"
            // We look for a pattern: contains '-' and ends with hex-like characters
            if file_name.contains('-') {
                if let Some(suffix) = file_name.rsplit('-').next() {
                    // Check if suffix looks like a hex hash (8+ hex characters)
                    if suffix.len() >= 8 && suffix.chars().all(|c| c.is_ascii_hexdigit()) {
                        self.is_test = true;
                    }
                }
            }
        }
    }

    /// Initialize template variables.
    fn init_template_vars(&mut self) {
        // Start with user-defined variables from config
        self.template_vars = self.config.variables.clone();

        // Add built-in variables
        self.template_vars.insert(
            "EXECUTABLE".to_string(),
            self.executable.display().to_string(),
        );

        if let Some(exe_name) = self.executable.file_name().and_then(|n| n.to_str()) {
            self.template_vars
                .insert("EXECUTABLE_NAME".to_string(), exe_name.to_string());
        }

        self.template_vars.insert(
            "WORKSPACE_ROOT".to_string(),
            self.workspace_root.display().to_string(),
        );

        self.template_vars.insert(
            "OUTPUT_DIR".to_string(),
            self.output_dir.display().to_string(),
        );

        self.template_vars.insert(
            "IS_TEST".to_string(),
            if self.is_test { "1" } else { "0" }.to_string(),
        );
    }

    /// Get the appropriate extra arguments based on whether this is a test run.
    pub fn get_extra_args(&self) -> &[String] {
        if self.is_test {
            &self.config.test.extra_args
        } else {
            &self.config.run.extra_args
        }
    }

    /// Get the success exit code for tests, if configured.
    pub fn test_success_exit_code(&self) -> Option<i32> {
        self.config.test.success_exit_code
    }
}
