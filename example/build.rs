// ! A simple build script to add the linker script to the build process

fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    // The linker script to use based on the target architecture
    let linker_file = format!("kernel-{}.ld", target);
    // Tell Cargo that if the linker file changes, to rerun this build script.
    println!("cargo:rerun-if-changed={}", linker_file);
    // Tell Cargo to pass the linker script to the linker
    println!("cargo:rustc-link-arg=-T{}", linker_file);
}
