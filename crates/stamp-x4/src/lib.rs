//! Xteink X4 hardware boundary.
//!
//! Phase 0 deliberately contains no register or peripheral initialization.
//! These constants are a reviewable transcription of the pinned FreeInk board
//! profile, not a working driver. The device's panel controller must be probed
//! before controller-specific commands or waveforms are used.

#![no_std]

use stamp_core::display::DisplayGeometry;

#[cfg(all(feature = "device", target_arch = "riscv32", target_os = "none"))]
pub mod input;

pub const DISPLAY_GEOMETRY: DisplayGeometry = DisplayGeometry {
    width: 800,
    height: 480,
};

pub mod reference_pins {
    pub const DISPLAY_SCLK: u8 = 8;
    pub const DISPLAY_MOSI: u8 = 10;
    pub const DISPLAY_CS: u8 = 21;
    pub const DISPLAY_DC: u8 = 4;
    pub const DISPLAY_RESET: u8 = 5;
    pub const DISPLAY_BUSY: u8 = 6;

    pub const SD_MISO: u8 = 7;
    pub const SD_CS: u8 = 12;

    pub const BUTTON_ADC_1: u8 = 1;
    pub const BUTTON_ADC_2: u8 = 2;
    pub const POWER_BUTTON: u8 = 3;
    pub const BATTERY_ADC: u8 = 0;
    pub const USB_DETECT: u8 = 20;
    pub const POWER_LATCH: u8 = 13;
}
