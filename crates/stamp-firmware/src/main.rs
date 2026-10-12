//! Linkable startup scaffold; not yet qualified for flashing on the X4.
#![no_std]
#![no_main]

// Supplies panic diagnostics over the USB Serial/JTAG peripheral.
use esp_backtrace as _;

// Application metadata for compatibility with the ESP-IDF bootloader format.
// This does not install ESP-IDF, an RTOS, or a new bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    // Initialize the HAL once; peripheral handles are unused in this first lesson.
    let _peripherals = esp_hal::init(esp_hal::Config::default());
    esp_println::println!("hello world");
    // Keep the entry function running: its return type is `!`.
    loop {
        core::hint::spin_loop();
    }
}
