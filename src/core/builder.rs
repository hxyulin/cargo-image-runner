use crate::bootloader::Bootloader;
use crate::config::{BootloaderKind, Config, ImageFormat, RunnerKind};
#[cfg(feature = "cargo-metadata")]
use crate::config::ConfigLoader;
use crate::core::context::Context;
use crate::core::error::{Error, Result};
use crate::image::ImageBuilder;
use crate::runner::io::IoHandler;
use crate::runner::{RunResult, Runner};
use std::path::PathBuf;

/// Builder for creating and running bootable images.
pub struct ImageRunnerBuilder {
    config: Option<Config>,
    workspace_root: Option<PathBuf>,
    executable: Option<PathBuf>,
    bootloader: Option<Box<dyn Bootloader>>,
    image_builder: Option<Box<dyn ImageBuilder>>,
    runner: Option<Box<dyn Runner>>,
    cli_extra_args: Vec<String>,
    io_handler: Option<Box<dyn IoHandler>>,
}

impl ImageRunnerBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            config: None,
            workspace_root: None,
            executable: None,
            bootloader: None,
            image_builder: None,
            runner: None,
            cli_extra_args: Vec::new(),
            io_handler: None,
        }
    }

    /// Set the configuration directly.
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Load configuration from Cargo.toml metadata.
    #[cfg(feature = "cargo-metadata")]
    pub fn from_cargo_metadata(mut self) -> Result<Self> {
        let (config, workspace_root) = ConfigLoader::new().load()?;
        self.config = Some(config);
        self.workspace_root = Some(workspace_root);
        Ok(self)
    }

    /// Load configuration from a standalone TOML file.
    #[cfg(feature = "cargo-metadata")]
    pub fn from_config_file(mut self, path: impl Into<PathBuf>) -> Result<Self> {
        let (config, workspace_root) = ConfigLoader::new().config_file(path).load()?;
        self.config = Some(config);
        self.workspace_root = Some(workspace_root);
        Ok(self)
    }

    /// Set the executable path.
    pub fn executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.executable = Some(path.into());
        self
    }

    /// Set the workspace root.
    pub fn workspace_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(path.into());
        self
    }

    /// Set extra QEMU arguments from CLI passthrough (`-- args`).
    pub fn extra_args(mut self, args: Vec<String>) -> Self {
        self.cli_extra_args = args;
        self
    }

    // --- Bootloader Configuration ---

    /// Set a custom bootloader implementation.
    pub fn bootloader<B: Bootloader + 'static>(mut self, bootloader: B) -> Self {
        self.bootloader = Some(Box::new(bootloader));
        self
    }

    /// Use Limine bootloader.
    #[cfg(feature = "limine")]
    pub fn limine(mut self) -> Self {
        self.bootloader = Some(Box::new(crate::bootloader::limine::LimineBootloader::new()));
        self
    }

    /// Use GRUB bootloader.
    pub fn grub(mut self) -> Self {
        self.bootloader = Some(Box::new(crate::bootloader::grub::GrubBootloader::new()));
        self
    }

    /// Use no bootloader (direct boot).
    pub fn no_bootloader(mut self) -> Self {
        self.bootloader = Some(Box::new(crate::bootloader::none::NoneBootloader::new()));
        self
    }

    // --- Image Format Configuration ---

    /// Set a custom image builder implementation.
    pub fn image_builder<I: ImageBuilder + 'static>(mut self, builder: I) -> Self {
        self.image_builder = Some(Box::new(builder));
        self
    }

    /// Build an ISO image.
    #[cfg(feature = "iso")]
    pub fn iso_image(mut self) -> Self {
        self.image_builder = Some(Box::new(crate::image::iso::IsoImageBuilder::new()));
        self
    }

    /// Build a FAT filesystem image.
    #[cfg(feature = "fat")]
    pub fn fat_image(mut self) -> Self {
        self.image_builder = Some(Box::new(crate::image::fat::FatImageBuilder::new()));
        self
    }

    /// Output to a directory (for QEMU fat:rw:).
    pub fn directory_output(mut self) -> Self {
        self.image_builder = Some(Box::new(crate::image::directory::DirectoryBuilder::new()));
        self
    }

    // --- Runner Configuration ---

    /// Set a custom runner implementation.
    pub fn runner<R: Runner + 'static>(mut self, runner: R) -> Self {
        self.runner = Some(Box::new(runner));
        self
    }

    /// Use QEMU runner.
    #[cfg(feature = "qemu")]
    pub fn qemu(mut self) -> Self {
        self.runner = Some(Box::new(crate::runner::qemu::QemuRunner::new()));
        self
    }

    // --- I/O Handler Configuration ---

    /// Set an I/O handler for serial capture/streaming.
    ///
    /// When set, the runner will pipe QEMU's serial output through the handler,
    /// enabling capture, pattern matching, and reactive input.
    pub fn io_handler<H: IoHandler + 'static>(mut self, handler: H) -> Self {
        self.io_handler = Some(Box::new(handler));
        self
    }

    // --- Build and Execute ---

    /// Build the image runner.
    pub fn build(self) -> Result<ImageRunner> {
        let config = self.config.ok_or_else(|| Error::config("no configuration provided"))?;

        let workspace_root = self.workspace_root.ok_or_else(|| {
            Error::config("workspace root not set (call from_cargo_metadata or workspace_root)")
        })?;

        let executable = self.executable.ok_or_else(|| {
            Error::config("executable not set (call executable or get from CLI args)")
        })?;

        // Create bootloader from config if not explicitly set
        let bootloader = if let Some(bl) = self.bootloader {
            bl
        } else {
            create_bootloader_from_config(&config)?
        };

        // Create image builder from config if not explicitly set
        let image_builder = if let Some(ib) = self.image_builder {
            ib
        } else {
            create_image_builder_from_config(&config)?
        };

        // Create runner from config if not explicitly set
        let runner = if let Some(r) = self.runner {
            r
        } else {
            create_runner_from_config(&config)?
        };

        Ok(ImageRunner {
            config,
            workspace_root,
            executable,
            bootloader,
            image_builder,
            runner,
            cli_extra_args: self.cli_extra_args,
            io_handler: self.io_handler,
        })
    }

    /// Build and immediately run.
    pub fn run(self) -> Result<()> {
        let runner = self.build()?;
        runner.run()
    }

    /// Build and immediately run, returning the full [`RunResult`].
    pub fn run_with_result(self) -> Result<RunResult> {
        let runner = self.build()?;
        runner.run_with_result()
    }
}

impl Default for ImageRunnerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Image runner that orchestrates the build and run process.
pub struct ImageRunner {
    config: Config,
    workspace_root: PathBuf,
    executable: PathBuf,
    bootloader: Box<dyn Bootloader>,
    image_builder: Box<dyn ImageBuilder>,
    runner: Box<dyn Runner>,
    cli_extra_args: Vec<String>,
    io_handler: Option<Box<dyn IoHandler>>,
}

impl ImageRunner {
    /// Build the image without running it.
    ///
    /// Returns the path to the built image.
    pub fn build_image(&self) -> Result<PathBuf> {
        // Create context
        let mut ctx = Context::new(
            self.config.clone(),
            self.workspace_root.clone(),
            self.executable.clone(),
        )?;
        ctx.cli_extra_args = self.cli_extra_args.clone();
        ctx.env_extra_args = crate::config::env::get_extra_qemu_args();

        // Update ARGS template variable with CLI args
        ctx.template_vars.insert(
            "ARGS".to_string(),
            ctx.cli_extra_args.join(" "),
        );

        // Validate all components
        self.bootloader.validate_config(&ctx)?;
        self.image_builder.validate_boot_type(&ctx)?;

        // Prepare bootloader files
        if ctx.config.verbose {
            println!("Preparing bootloader: {}", self.bootloader.name());
        }
        let bootloader_files = self.bootloader.prepare(&ctx)?;

        // Get config files and process templates
        let config_files = self.bootloader.config_files(&ctx)?;
        let mut all_files = Vec::new();

        // Add bootloader files
        all_files.extend(bootloader_files.bios_files);
        all_files.extend(bootloader_files.uefi_files);
        all_files.extend(bootloader_files.system_files);

        // Process config files with templates
        for config_file in config_files {
            if config_file.needs_template_processing {
                // Read, process, and write template
                let content = std::fs::read_to_string(&config_file.source)?;
                let processed = self.bootloader.process_templates(&content, &ctx.template_vars)?;

                // Write to temporary file
                let temp_path = ctx.output_dir.join("processed_config");
                std::fs::create_dir_all(&temp_path)?;
                let processed_file = temp_path.join(
                    config_file
                        .source
                        .file_name()
                        .ok_or_else(|| Error::config("invalid config file path"))?,
                );
                std::fs::write(&processed_file, processed)?;

                all_files.push(crate::bootloader::FileEntry::new(
                    processed_file,
                    config_file.dest,
                ));
            } else {
                all_files.push(crate::bootloader::FileEntry::new(
                    config_file.source,
                    config_file.dest,
                ));
            }
        }

        // Add extra files from config
        all_files.extend(collect_extra_files(&ctx)?);

        // Build image
        if ctx.config.verbose {
            println!("Building image: {}", self.image_builder.name());
        }
        let image_path = self.image_builder.build(&ctx, &all_files)?;

        Ok(image_path)
    }

    /// Run the full pipeline: prepare bootloader, build image, execute.
    ///
    /// Returns `Ok(())` on success or an error on failure.
    /// If an I/O handler was set, it will be used during execution.
    pub fn run(self) -> Result<()> {
        let result = self.run_with_result()?;

        // The run_with_result already checks exit codes, so if we get here
        // it means the run was considered successful or we need to check.
        // However, run_with_result returns the raw result, so we check here.
        if result.timed_out {
            return Err(Error::runner("test timed out"));
        }

        if !result.success {
            return Err(Error::runner(format!(
                "Execution failed with exit code: {}",
                result.exit_code
            )));
        }

        Ok(())
    }

    /// Run the full pipeline and return the detailed [`RunResult`].
    ///
    /// Unlike [`run()`](Self::run), this does not error on non-zero exit codes.
    /// The caller can inspect the result to determine success/failure.
    /// If an I/O handler was set, captured output will be available in the result.
    pub fn run_with_result(mut self) -> Result<RunResult> {
        // Create context
        let mut ctx = Context::new(self.config, self.workspace_root, self.executable)?;
        ctx.cli_extra_args = self.cli_extra_args;
        ctx.env_extra_args = crate::config::env::get_extra_qemu_args();

        // Update ARGS template variable with CLI args
        ctx.template_vars.insert(
            "ARGS".to_string(),
            ctx.cli_extra_args.join(" "),
        );

        // Validate all components
        self.bootloader.validate_config(&ctx)?;
        self.image_builder.validate_boot_type(&ctx)?;
        self.runner.validate(&ctx)?;

        // Prepare bootloader files
        if ctx.config.verbose {
            println!("Preparing bootloader: {}", self.bootloader.name());
        }
        let bootloader_files = self.bootloader.prepare(&ctx)?;

        // Get config files and process templates
        let config_files = self.bootloader.config_files(&ctx)?;
        let mut all_files = Vec::new();

        // Add bootloader files
        all_files.extend(bootloader_files.bios_files);
        all_files.extend(bootloader_files.uefi_files);
        all_files.extend(bootloader_files.system_files);

        // Process config files with templates
        for config_file in config_files {
            if config_file.needs_template_processing {
                let content = std::fs::read_to_string(&config_file.source)?;
                let processed = self.bootloader.process_templates(&content, &ctx.template_vars)?;

                let temp_path = ctx.output_dir.join("processed_config");
                std::fs::create_dir_all(&temp_path)?;
                let processed_file = temp_path.join(
                    config_file
                        .source
                        .file_name()
                        .ok_or_else(|| Error::config("invalid config file path"))?,
                );
                std::fs::write(&processed_file, processed)?;

                all_files.push(crate::bootloader::FileEntry::new(
                    processed_file,
                    config_file.dest,
                ));
            } else {
                all_files.push(crate::bootloader::FileEntry::new(
                    config_file.source,
                    config_file.dest,
                ));
            }
        }

        // Add extra files from config
        all_files.extend(collect_extra_files(&ctx)?);

        // Build image
        if ctx.config.verbose {
            println!("Building image: {}", self.image_builder.name());
        }
        let image_path = self.image_builder.build(&ctx, &all_files)?;

        // Run image
        if ctx.config.verbose {
            println!("Running with: {}", self.runner.name());
        }

        let mut result = if let Some(ref mut handler) = self.io_handler {
            self.runner.run_with_io(&ctx, &image_path, handler.as_mut())?
        } else {
            self.runner.run(&ctx, &image_path)?
        };

        // Populate captured_output from handler.finish() if available
        if let Some(handler) = self.io_handler {
            if let Some(captured_io) = handler.finish() {
                let serial_str = String::from_utf8_lossy(&captured_io.serial).into_owned();
                result = result.with_serial(serial_str);
            }
        }

        Ok(result)
    }
}

// --- Extra Files ---

/// Collect extra files specified in config, resolving source paths relative to workspace root.
fn collect_extra_files(ctx: &Context) -> Result<Vec<crate::bootloader::FileEntry>> {
    let mut files = Vec::new();
    for (dest, src) in &ctx.config.extra_files {
        let source_path = ctx.workspace_root.join(src);
        if !source_path.exists() {
            return Err(Error::config(format!(
                "extra file not found: {} (resolved to {})",
                src,
                source_path.display()
            )));
        }
        // Strip leading '/' so dest is always relative to image root.
        // Users may write "/boot/file" meaning "boot/file within the image".
        let dest_path = PathBuf::from(dest.strip_prefix('/').unwrap_or(dest));
        files.push(crate::bootloader::FileEntry::new(
            source_path,
            dest_path,
        ));
    }
    Ok(files)
}

// --- Factory Functions ---

/// Create a bootloader from configuration.
fn create_bootloader_from_config(config: &Config) -> Result<Box<dyn Bootloader>> {
    match config.bootloader.kind {
        #[cfg(feature = "limine")]
        BootloaderKind::Limine => Ok(Box::new(crate::bootloader::limine::LimineBootloader::new())),

        #[cfg(not(feature = "limine"))]
        BootloaderKind::Limine => Err(Error::feature_not_enabled("limine")),

        BootloaderKind::Grub => Ok(Box::new(crate::bootloader::grub::GrubBootloader::new())),

        BootloaderKind::None => Ok(Box::new(crate::bootloader::none::NoneBootloader::new())),
    }
}

/// Create an image builder from configuration.
fn create_image_builder_from_config(config: &Config) -> Result<Box<dyn ImageBuilder>> {
    match config.image.format {
        #[cfg(feature = "iso")]
        ImageFormat::Iso => Ok(Box::new(crate::image::iso::IsoImageBuilder::new())),

        #[cfg(not(feature = "iso"))]
        ImageFormat::Iso => Err(Error::feature_not_enabled("iso")),

        #[cfg(feature = "fat")]
        ImageFormat::Fat => Ok(Box::new(crate::image::fat::FatImageBuilder::new())),

        #[cfg(not(feature = "fat"))]
        ImageFormat::Fat => Err(Error::feature_not_enabled("fat")),

        ImageFormat::Directory => Ok(Box::new(crate::image::directory::DirectoryBuilder::new())),
    }
}

/// Create a runner from configuration.
fn create_runner_from_config(config: &Config) -> Result<Box<dyn Runner>> {
    match config.runner.kind {
        #[cfg(feature = "qemu")]
        RunnerKind::Qemu => Ok(Box::new(crate::runner::qemu::QemuRunner::new())),

        #[cfg(not(feature = "qemu"))]
        RunnerKind::Qemu => Err(Error::feature_not_enabled("qemu")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_error_missing_config() {
        let result = ImageRunnerBuilder::new()
            .workspace_root("/tmp")
            .executable("/tmp/kernel")
            .build();
        let err = result.err().expect("should fail");
        assert!(err.to_string().contains("no configuration"));
    }

    #[test]
    fn test_builder_error_missing_workspace_root() {
        let result = ImageRunnerBuilder::new()
            .with_config(Config::default())
            .executable("/tmp/kernel")
            .build();
        let err = result.err().expect("should fail");
        assert!(err.to_string().contains("workspace root"));
    }

    #[test]
    fn test_builder_error_missing_executable() {
        let result = ImageRunnerBuilder::new()
            .with_config(Config::default())
            .workspace_root("/tmp")
            .build();
        let err = result.err().expect("should fail");
        assert!(err.to_string().contains("executable"));
    }

    #[test]
    fn test_builder_with_none_bootloader_and_directory() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("kernel");
        std::fs::write(&exe, b"fake").unwrap();

        // Config defaults: BootloaderKind::None, ImageFormat::Directory
        let result = ImageRunnerBuilder::new()
            .with_config(Config::default())
            .workspace_root(dir.path())
            .executable(&exe)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_explicit_components() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("kernel");
        std::fs::write(&exe, b"fake").unwrap();

        let result = ImageRunnerBuilder::new()
            .with_config(Config::default())
            .workspace_root(dir.path())
            .executable(&exe)
            .no_bootloader()
            .directory_output()
            .build();
        assert!(result.is_ok());
    }
}
