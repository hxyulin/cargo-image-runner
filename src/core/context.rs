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

    /// Extra QEMU arguments from CLI passthrough (`-- args`).
    pub cli_extra_args: Vec<String>,

    /// Extra QEMU arguments from `CARGO_IMAGE_RUNNER_QEMU_ARGS` env var.
    pub env_extra_args: Vec<String>,
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
            cli_extra_args: Vec::new(),
            env_extra_args: Vec::new(),
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
    /// Uses `file_stem()` to strip extensions like `.efi` before checking.
    pub fn detect_test(&mut self) {
        if let Some(stem) = self.executable.file_stem().and_then(|n| n.to_str()) {
            // Test binaries typically have a hash suffix like "mytest-a1b2c3d4"
            // We look for a pattern: contains '-' and ends with hex-like characters
            if stem.contains('-') {
                if let Some(suffix) = stem.rsplit('-').next() {
                    // Check if suffix looks like a hex hash (8+ hex characters)
                    if suffix.len() >= 8 && suffix.chars().all(|c| c.is_ascii_hexdigit()) {
                        self.is_test = true;
                    }
                }
            }
        }
    }

    /// Initialize template variables.
    ///
    /// Layering order:
    /// 1. Config variables (`[variables]`)
    /// 2. Env var variables (`CARGO_IMAGE_RUNNER_VAR_*`)
    /// 3. Built-in variables (always win)
    fn init_template_vars(&mut self) {
        // 1. Start with user-defined variables from config
        self.template_vars = self.config.variables.clone();

        // 2. Overlay env var variables (override config vars)
        for (key, value) in crate::config::env::collect_env_variables() {
            self.template_vars.insert(key, value);
        }

        // 3. Built-in variables (always override everything)
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

        // ARGS: CLI extra args joined with spaces (for kernel command line).
        // Initialized empty here; populated later when cli_extra_args are available.
        self.template_vars
            .insert("ARGS".to_string(), String::new());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn make_context(workspace: &std::path::Path, exe: &std::path::Path) -> Context {
        Context::new(Config::default(), workspace.to_path_buf(), exe.to_path_buf()).unwrap()
    }

    #[test]
    fn test_context_paths() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let ctx = make_context(dir.path(), &exe);
        assert_eq!(ctx.target_dir, dir.path().join("target/image-runner"));
        assert_eq!(ctx.cache_dir, dir.path().join("target/image-runner/cache"));
        assert_eq!(
            ctx.output_dir,
            dir.path().join("target/image-runner/output")
        );
    }

    #[test]
    fn test_builtin_template_vars() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let ctx = make_context(dir.path(), &exe);
        assert_eq!(
            ctx.template_vars.get("EXECUTABLE").unwrap(),
            &exe.display().to_string()
        );
        assert_eq!(
            ctx.template_vars.get("EXECUTABLE_NAME").unwrap(),
            "my-kernel"
        );
        assert_eq!(
            ctx.template_vars.get("WORKSPACE_ROOT").unwrap(),
            &dir.path().display().to_string()
        );
        assert_eq!(
            ctx.template_vars.get("OUTPUT_DIR").unwrap(),
            &ctx.output_dir.display().to_string()
        );
        assert_eq!(ctx.template_vars.get("IS_TEST").unwrap(), "0");
    }

    #[test]
    fn test_user_variables_included() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let mut config = Config::default();
        config
            .variables
            .insert("MY_VAR".to_string(), "hello".to_string());

        let ctx =
            Context::new(config, dir.path().to_path_buf(), exe).unwrap();
        assert_eq!(ctx.template_vars.get("MY_VAR").unwrap(), "hello");
    }

    #[test]
    fn test_detect_test_with_hash_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-test-a1b2c3d4e5f6a7b8");
        std::fs::write(&exe, b"fake").unwrap();

        let ctx = make_context(dir.path(), &exe);
        assert!(ctx.is_test);
        assert_eq!(ctx.template_vars.get("IS_TEST").unwrap(), "1");
    }

    #[test]
    fn test_detect_test_with_efi_extension() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("basic_boot-edba05eea98a559f.efi");
        std::fs::write(&exe, b"fake").unwrap();

        let ctx = make_context(dir.path(), &exe);
        assert!(ctx.is_test);
        assert_eq!(ctx.template_vars.get("IS_TEST").unwrap(), "1");
    }

    #[test]
    fn test_detect_normal_executable() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let ctx = make_context(dir.path(), &exe);
        assert!(!ctx.is_test);
    }

    #[test]
    fn test_get_extra_args_test_mode() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-test-a1b2c3d4e5f6a7b8");
        std::fs::write(&exe, b"fake").unwrap();

        let mut config = Config::default();
        config.test.extra_args = vec!["-device".to_string(), "isa-debug-exit".to_string()];
        config.run.extra_args = vec!["-serial".to_string(), "stdio".to_string()];

        let ctx =
            Context::new(config, dir.path().to_path_buf(), exe).unwrap();
        assert!(ctx.is_test);
        assert_eq!(ctx.get_extra_args(), &["-device", "isa-debug-exit"]);
    }

    #[test]
    fn test_get_extra_args_run_mode() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let mut config = Config::default();
        config.test.extra_args = vec!["-device".to_string(), "isa-debug-exit".to_string()];
        config.run.extra_args = vec!["-serial".to_string(), "stdio".to_string()];

        let ctx =
            Context::new(config, dir.path().to_path_buf(), exe).unwrap();
        assert!(!ctx.is_test);
        assert_eq!(ctx.get_extra_args(), &["-serial", "stdio"]);
    }

    #[test]
    fn test_success_exit_code() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let mut config = Config::default();
        config.test.success_exit_code = Some(33);

        let ctx =
            Context::new(config, dir.path().to_path_buf(), exe).unwrap();
        assert_eq!(ctx.test_success_exit_code(), Some(33));
    }

    #[test]
    fn test_env_variables_override_config() {
        use std::sync::Mutex;
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = LOCK.lock().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let mut config = Config::default();
        config.variables.insert("MYVAR".to_string(), "from_config".to_string());

        // Set env var that should override
        let env_key = "CARGO_IMAGE_RUNNER_VAR_MYVAR";
        let old = std::env::var(env_key).ok();
        // SAFETY: test is serialized via LOCK
        unsafe { std::env::set_var(env_key, "from_env") };

        let ctx = Context::new(config, dir.path().to_path_buf(), exe).unwrap();
        assert_eq!(ctx.template_vars.get("MYVAR").unwrap(), "from_env");

        // SAFETY: test is serialized via LOCK
        match old {
            Some(v) => unsafe { std::env::set_var(env_key, v) },
            None => unsafe { std::env::remove_var(env_key) },
        }
    }

    #[test]
    fn test_builtin_vars_override_env_vars() {
        use std::sync::Mutex;
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = LOCK.lock().unwrap();

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("my-kernel");
        std::fs::write(&exe, b"fake").unwrap();

        // Try to override a built-in var via env — it should NOT succeed
        let env_key = "CARGO_IMAGE_RUNNER_VAR_EXECUTABLE_NAME";
        let old = std::env::var(env_key).ok();
        // SAFETY: test is serialized via LOCK
        unsafe { std::env::set_var(env_key, "should_not_win") };

        let ctx = Context::new(Config::default(), dir.path().to_path_buf(), exe).unwrap();
        // Built-in EXECUTABLE_NAME should win
        assert_eq!(ctx.template_vars.get("EXECUTABLE_NAME").unwrap(), "my-kernel");

        // SAFETY: test is serialized via LOCK
        match old {
            Some(v) => unsafe { std::env::set_var(env_key, v) },
            None => unsafe { std::env::remove_var(env_key) },
        }
    }
}
