//! First button-ladder exercise: raw ADC1 readings from GPIO1.
//!

// Public re-exports keep this initial scaffold compilable without unused imports.
pub use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcConfig, AdcPin, Attenuation},
    peripherals::{ADC1, GPIO1},
};

/// This struct is to help us figure out which buttons are pressed and released.
///
/// It contains a pin that we read values on + an analog to digital converter that
/// converts the raw voltages to some application specific numbers.
///
/// e.g. We can assign things like pin1 + reading 42 -> left button.
pub struct ButtonContext<'a> {
    /// The pin we are doing the readings on.
    pin: AdcPin<GPIO1<'a>, ADC1<'a>>,
    /// The analog to digital converter in the board that gives us
    /// readings on the pin that we can use to recognize specific buttons.
    adc: Adc<'a, ADC1<'a>, Blocking>,
}

impl<'a> ButtonContext<'a> {
    pub fn new(peripheral: ADC1<'a>, pin: GPIO1<'a>) -> Self {
        let mut config: _ = AdcConfig::new();
        let pin = config.enable_pin(pin, Attenuation::_11dB);
        let adc = Adc::new(peripheral, config);
        Self { pin, adc }
    }

    pub fn read_value(&mut self) -> nb::Result<u16, ()> {
        self.adc.read_oneshot(&mut self.pin)
    }
}
