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
    // Give the button context ADC1, both analog inputs, and the digital power input.
    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_println::println!("hello world");
    let mut button_context = ButtonContext::new(
        peripherals.ADC1,
        peripherals.GPIO1,
        peripherals.GPIO2,
        peripherals.GPIO3,
    );
    let delay = Delay::new();
    loop {
        // Allow up to 10 ms for a complete snapshot. ButtonContext retains
        // partial progress if we pause and retry on the next iteration.
        let started = Instant::now();
        loop {
            match button_context.poll_buttons() {
                Ok([front, side, power]) => {
                    esp_println::println!(
                        "Buttons: front={:?}, side={:?}, power={:?}",
                        front,
                        side,
                        power
                    );
                    break;
                }
                Err(nb::Error::WouldBlock) => {
                    if started.elapsed() >= Duration::from_millis(10) {
                        esp_println::println!("Button snapshot pending; retrying after delay");
                        break;
                    }
                    // Conversion is pending; poll again without a long delay.
                }
                Err(nb::Error::Other(error)) => {
                    esp_println::println!("Button read error: {:?}", error);
                    break;
                }
            }
        }

        // Pace every attempt, whether it succeeded, failed, or timed out.
        delay.delay_millis(100);
    }
}
