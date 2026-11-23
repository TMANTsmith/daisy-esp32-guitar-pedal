fn main() {
    // Add the RustMeter linker script
    println!("cargo:rustc-link-arg=-Trustmeter.x");
}
