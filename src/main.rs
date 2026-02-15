use cargo_image_runner::builder;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cargo-image-runner")]
#[command(about = "Build and run bootable images for embedded/kernel development")]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to the executable to run (when no subcommand is given)
    #[arg(value_name = "EXECUTABLE")]
    executable: Option<PathBuf>,

    /// Additional QEMU arguments (when no subcommand is given)
    #[clap(last = true)]
    qemu_args: Vec<String>,
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
        Some(Command::Run { executable, qemu_args: _ }) => {
            run_executable(executable)?;
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
                run_executable(executable)?;
            } else {
                eprintln!("Error: No executable specified");
                eprintln!("Usage: cargo-image-runner <EXECUTABLE>");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn run_executable(executable: PathBuf) -> cargo_image_runner::Result<()> {
    builder()
        .from_cargo_metadata()?
        .executable(executable)
        .run()
}

fn build_image(executable: PathBuf) -> cargo_image_runner::Result<()> {
    // Load configuration
    let (config, _workspace_root) = cargo_image_runner::config::ConfigLoader::new().load()?;

    println!("Building image for: {}", executable.display());
    println!("Boot type: {:?}", config.boot.boot_type);
    println!("Bootloader: {:?}", config.bootloader.kind);
    println!("Image format: {:?}", config.image.format);

    // Build the image using the builder
    println!("\n--- Building image runner ---");
    let runner = builder()
        .from_cargo_metadata()?
        .executable(executable)
        .build()?;

    println!("--- Executing build ---");
    let image_path = runner.build_image()?;
    println!("\n✓ Image built successfully at: {}", image_path.display());

    Ok(())
}

fn clean_artifacts() -> cargo_image_runner::Result<()> {
    use cargo_image_runner::config::ConfigLoader;

    // Load configuration to find the target directory
    let (_, workspace_root) = ConfigLoader::new().load()?;
    let target_dir = workspace_root.join("target").join("image-runner");

    if target_dir.exists() {
        println!("Cleaning artifacts in: {}", target_dir.display());
        std::fs::remove_dir_all(&target_dir)?;
        println!("Cleaned successfully");
    } else {
        println!("No artifacts to clean");
    }

    Ok(())
}

fn check_config() -> cargo_image_runner::Result<()> {
    use cargo_image_runner::config::ConfigLoader;

    println!("Checking configuration...");

    // Load configuration
    let (config, workspace_root) = ConfigLoader::new().load()?;

    println!("Workspace root: {}", workspace_root.display());
    println!("Boot type: {:?}", config.boot.boot_type);
    println!("Bootloader: {:?}", config.bootloader.kind);
    println!("Image format: {:?}", config.image.format);
    println!("Runner: {:?}", config.runner.kind);

    // Check QEMU availability
    #[cfg(feature = "qemu")]
    {
        use std::process::Command;
        let qemu_available = Command::new(&config.runner.qemu.binary)
            .arg("--version")
            .output()
            .is_ok();

        if qemu_available {
            println!("✓ QEMU available: {}", config.runner.qemu.binary);
        } else {
            println!("✗ QEMU not found: {}", config.runner.qemu.binary);
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
            println!("✓ Limine config found: {}", limine_conf.display());
        } else {
            println!("✗ Limine config not found: {}", limine_conf.display());
        }
    }

    println!("\nConfiguration check complete");

    Ok(())
}

fn print_version() {
    println!("cargo-image-runner {}", env!("CARGO_PKG_VERSION"));
}
