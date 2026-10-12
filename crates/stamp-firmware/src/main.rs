//! Linkable startup scaffold; not yet qualified for flashing on the X4.
#![no_std]
#![no_main]

// Supplies panic diagnostics over the USB Serial/JTAG peripheral.
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    time::{Duration, Instant},
};
use stamp_x4::input::ButtonContext;

// Application metadata for compatibility with the ESP-IDF bootloader format.
// This does not install ESP-IDF, an RTOS, or a new bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    // Initialize the HAL once, then give ADC1 and GPIO1 to the button context.
    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_println::println!("hello world");
    let mut button_context = ButtonContext::new(peripherals.ADC1, peripherals.GPIO1);
    let delay = Delay::new();

    loop {
        // Give this measurement attempt its own 10 ms deadline.
        let started = Instant::now();
        loop {
            match button_context.read_value() {
                Ok(value) => {
                    esp_println::println!("GPIO1 raw: {}", value);
                    break;
                }
                Err(nb::Error::WouldBlock) => {
                    if started.elapsed() >= Duration::from_millis(10) {
                        esp_println::println!("ADC read timed out");
                        break;
                    }
                    // Conversion is pending; poll again without a long delay.
                }
                Err(nb::Error::Other(error)) => {
                    esp_println::println!("ADC read error: {:?}", error);
                    break;
                }
            }
        }

        // Pace every attempt, whether it succeeded, failed, or timed out.
        delay.delay_millis(100);
    }
}
