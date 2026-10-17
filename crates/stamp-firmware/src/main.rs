//! USB-powered display diagnostics and debounced button events.
#![no_std]
#![no_main]

// Supplies panic diagnostics over the USB Serial/JTAG peripheral.
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    time::{Duration, Instant},
};
use stamp_core::{
    display::{MonochromeFramebuffer, draw_message},
    input::Debouncer,
};
use stamp_x4::display::{DisplayDriver, DisplayError};
use stamp_x4::input::ButtonContext;

// Application metadata for compatibility with the ESP-IDF bootloader format.
// This does not install ESP-IDF, an RTOS, or a new bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

fn display_test<'a>(
    display: &mut DisplayDriver<'a>,
    framebuffer: &mut MonochromeFramebuffer<800, 480, 48000>,
) -> Result<(), DisplayError> {
    esp_println::println!("Initializing display");
    display.init_ssd1677()?;
    esp_println::println!("Successfully initialized display");

    esp_println::println!("Writing test pattern");
    display.words(framebuffer.data())?;
    esp_println::println!("Successfully wrote test pattern");

    esp_println::println!("Refreshing display");
    display.refresh_full()?;
    esp_println::println!("Successfully refreshed display");

    Ok(())
}

#[esp_hal::main]
fn main() -> ! {
    // Give the button context ADC1, both analog inputs, and the digital power input.
    let peripherals = esp_hal::init(esp_hal::Config::default());
    esp_println::println!("Stamp input: 5 ms sampling, 20 ms debounce");
    let mut button_context = ButtonContext::new(
        peripherals.ADC1,
        peripherals.GPIO1,
        peripherals.GPIO2,
        peripherals.GPIO3,
    );
    let mut debouncer = Debouncer::new();
    let input_started = Instant::now();
    let delay = Delay::new();
    // Retain the driver so its owned pins remain configured after the probe.
    let mut display = DisplayDriver::new(
        peripherals.SPI2,
        peripherals.GPIO8,
        peripherals.GPIO10,
        peripherals.GPIO21,
        peripherals.GPIO4,
        peripherals.GPIO5,
        peripherals.GPIO6,
        peripherals.GPIO12,
    )
    .unwrap();

    let mut fb: MonochromeFramebuffer<800, 480, 48000> = MonochromeFramebuffer::new();
    draw_message(&mut fb);
    if let Err(e) = display_test(&mut display, &mut fb) {
        esp_println::println!("Failed display test {:?}", e);
    }
    loop {
        // Allow up to 10 ms for a complete snapshot. ButtonContext retains
        // partial progress if we pause and retry on the next iteration.
        let started = Instant::now();
        loop {
            match button_context.poll_buttons() {
                Ok(snapshot) => {
                    // Only complete snapshots reach the debouncer. Pending reads
                    // must not be interpreted as released buttons.
                    let now_ms = input_started.elapsed().as_millis();
                    for event in debouncer.update(snapshot, now_ms).into_iter().flatten() {
                        esp_println::println!("[{} ms] {:?}", now_ms, event);
                    }
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
        delay.delay_millis(5);
    }
}
