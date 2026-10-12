fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_DEVICE").is_some() {
        let target = std::env::var("TARGET").expect("Cargo supplies TARGET");
        assert_eq!(
            target, "riscv32imc-unknown-none-elf",
            "The device feature requires --target riscv32imc-unknown-none-elf; use cargo firmware"
        );
        // Apply the HAL's memory layout only to the executable, not host tests.
        println!("cargo:rustc-link-arg-bin=stamp-firmware=-Tlinkall.x");
    }
}
