use cargo_image_runner::builder;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cargo-image-runner")]
#[command(about = "Build and run bootable images for embedded/kernel development")]
#[command(version)]
#[command(trailing_var_arg = true)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to the executable to run (when no subcommand is given)
    #[arg(value_name = "EXECUTABLE")]
    executable: Option<PathBuf>,

    /// Additional arguments passed by cargo (forwarded to kernel in test mode, QEMU in run mode)
    #[arg(allow_hyphen_values = true)]
    extra_args: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run an executable (default command)
    Run {
        /// Path to the executable
        executable: PathBuf,

        /// Additional QEMU arguments
        #[clap(last = true)]
        qemu_args: Vec<String>,
    },

    /// Build the image without running
    Build {
        /// Path to the executable
        executable: PathBuf,
    },

    /// Clean build artifacts
    Clean,

    /// Check configuration and dependencies
    Check,

    /// Print version information
    Version,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> cargo_image_runner::Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::Run { executable, qemu_args }) => {
            run_executable(executable, qemu_args)?;
        }
        Some(Command::Build { executable }) => {
            build_image(executable)?;
        }
        Some(Command::Clean) => {
            clean_artifacts()?;
        }
        Some(Command::Check) => {
            check_config()?;
        }
        Some(Command::Version) => {
            print_version();
        }
        None => {
            // Default behavior: run the executable
            if let Some(executable) = args.executable {
                run_executable(executable, args.extra_args)?;
            } else {
                eprintln!("Error: No executable specified");
                eprintln!("Usage: cargo-image-runner <EXECUTABLE>");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn run_executable(executable: PathBuf, qemu_args: Vec<String>) -> cargo_image_runner::Result<()> {
    builder()
        .from_cargo_metadata()?
        .executable(executable)
        .extra_args(qemu_args)
        .run()
}

fn build_image(executable: PathBuf) -> cargo_image_runner::Result<()> {
    // Load configuration
    let (config, _workspace_root) = cargo_image_runner::config::ConfigLoader::new().load()?;

    let verbose = config.verbose;

    if verbose {
        println!("Building image for: {}", executable.display());
        println!("Boot type: {:?}", config.boot.boot_type);
        println!("Bootloader: {:?}", config.bootloader.kind);
        println!("Image format: {:?}", config.image.format);
    }

    // Build the image using the builder
    if verbose {
        println!("\n--- Building image runner ---");
    }
    let runner = builder()
        .from_cargo_metadata()?
        .executable(executable)
        .build()?;

    if verbose {
        println!("--- Executing build ---");
    }
    let image_path = runner.build_image()?;
    if verbose {
        println!("\n✓ Image built successfully at: {}", image_path.display());
    }

    Ok(())
}

fn clean_artifacts() -> cargo_image_runner::Result<()> {
    use cargo_image_runner::config::ConfigLoader;

    // Load configuration to find the target directory
    let (config, workspace_root) = ConfigLoader::new().load()?;
    let target_dir = workspace_root.join("target").join("image-runner");

    if target_dir.exists() {
        if config.verbose {
            println!("Cleaning artifacts in: {}", target_dir.display());
        }
        std::fs::remove_dir_all(&target_dir)?;
        if config.verbose {
            println!("Cleaned successfully");
        }
    } else if config.verbose {
        println!("No artifacts to clean");
    }

    Ok(())
}

fn check_config() -> cargo_image_runner::Result<()> {
    use cargo_image_runner::config::ConfigLoader;
    use cargo_image_runner::config::env;

    println!("Checking configuration...");

    // Load configuration (includes profile + env overrides)
    let (config, workspace_root) = ConfigLoader::new().load()?;

    println!("Workspace root: {}", workspace_root.display());
    println!("Boot type: {:?}", config.boot.boot_type);
    println!("Bootloader: {:?}", config.bootloader.kind);
    println!("Image format: {:?}", config.image.format);
    println!("Runner: {:?}", config.runner.kind);
    println!("Verbose: {}", config.verbose);

    // Show active profile
    if let Some(profile) = env::get_profile_name() {
        println!("\nActive profile: {}", profile);
    }

    // Show env var overrides
    let overrides = env::detect_active_overrides();
    if !overrides.is_empty() {
        println!("\nEnvironment variable overrides:");
        for (key, value) in &overrides {
            println!("  {} = {}", key, value);
        }
    }

    // Show env var template variables
    let env_vars = env::collect_env_variables();
    if !env_vars.is_empty() {
        println!("\nTemplate variables from environment:");
        for (key, value) in &env_vars {
            println!("  {} = {}", key, value);
        }
    }

    // Show extra files
    if !config.extra_files.is_empty() {
        println!("\nExtra files:");
        for (dest, src) in &config.extra_files {
            println!("  {} <- {}", dest, src);
        }
    }

    // Check QEMU availability
    #[cfg(feature = "qemu")]
    {
        use std::process::Command;
        let qemu_available = Command::new(&config.runner.qemu.binary)
            .arg("--version")
            .output()
            .is_ok();

        if qemu_available {
            println!("QEMU available: {}", config.runner.qemu.binary);
        } else {
            println!("QEMU not found: {}", config.runner.qemu.binary);
        }

        println!("  Memory: {} MB", config.runner.qemu.memory);
        println!("  Cores: {}", config.runner.qemu.cores);
        println!("  Machine: {}", config.runner.qemu.machine);
        println!("  KVM: {}", config.runner.qemu.kvm);
        if !config.runner.qemu.extra_args.is_empty() {
            println!("  Extra args: {:?}", config.runner.qemu.extra_args);
        }
    }

    // Check for Limine config if using Limine
    #[cfg(feature = "limine")]
    if let cargo_image_runner::config::BootloaderKind::Limine = config.bootloader.kind {
        let limine_conf = if let Some(ref path) = config.bootloader.config_file {
            workspace_root.join(path)
        } else {
            workspace_root.join("limine.conf")
        };

        if limine_conf.exists() {
            println!("Limine config found: {}", limine_conf.display());
        } else {
            println!("Limine config not found: {}", limine_conf.display());
        }
    }

    println!("\nConfiguration check complete");

    Ok(())
}

fn print_version() {
    println!("cargo-image-runner {}", env!("CARGO_PKG_VERSION"));
}
