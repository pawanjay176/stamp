//! Button snapshots from the two ADC ladders and digital power input.

use esp_hal::{
    analog::adc::AdcCalBasic,
    gpio::{Input, InputConfig},
    peripherals::{GPIO2, GPIO3},
};
// Public re-exports keep this initial scaffold compilable without unused imports.
pub use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcConfig, AdcPin, Attenuation},
    peripherals::{ADC1, GPIO1},
};
use stamp_core::input::Button;

enum ReadState {
    /// Started reading, haven't received a value from front yet.
    Started,
    /// Started and have received a value from front.
    Front { val: Option<Button> },
}

/// This struct is to help us figure out which buttons are pressed and released.
///
/// It contains a pin that we read values on + an analog to digital converter that
/// converts the raw voltages to some application specific numbers.
///
/// e.g. We can assign things like pin1 + reading 42 -> left button.
pub struct ButtonContext<'a> {
    /// The general purpose IO pin that take readings from the front 4 buttons on the XTeink 4.
    pin_front: AdcPin<GPIO1<'a>, ADC1<'a>, AdcCalBasic<ADC1<'a>>>,
    /// The general purpose IO pin that take readings from side 2 buttons on the XTeink 4.
    pin_side: AdcPin<GPIO2<'a>, ADC1<'a>, AdcCalBasic<ADC1<'a>>>,
    /// Power button is a digital on and off switch with no analog range of values.
    power_button: Input<'a>,
    /// The analog to digital converter in the board that gives us
    /// readings on the pin that we can use to recognize specific buttons.
    adc: Adc<'a, ADC1<'a>, Blocking>,
    read_state: ReadState,
}

impl<'a> ButtonContext<'a> {
    pub fn new(
        peripheral: ADC1<'a>,
        pin_front: GPIO1<'a>,
        pin_side: GPIO2<'a>,
        power: GPIO3<'a>,
    ) -> Self {
        let mut config: _ = AdcConfig::new();
        let pin_front = config.enable_pin_with_cal(pin_front, Attenuation::_11dB);
        let pin_side = config.enable_pin_with_cal(pin_side, Attenuation::_11dB);
        // Pull::Up implies the button is not pressed and that is the default state
        let input_config = InputConfig::default().with_pull(esp_hal::gpio::Pull::Up);
        let power_button = Input::new(power, input_config);
        let adc = Adc::new(peripheral, config);
        Self {
            pin_front,
            pin_side,
            power_button,
            adc,
            read_state: ReadState::Started,
        }
    }

    pub fn read_value_front(&mut self) -> nb::Result<u16, ()> {
        self.adc.read_oneshot(&mut self.pin_front)
    }

    pub fn read_value_side(&mut self) -> nb::Result<u16, ()> {
        self.adc.read_oneshot(&mut self.pin_side)
    }

    /// Returns `true` if the power button is pressed.
    pub fn read_power(&self) -> bool {
        self.power_button.is_low()
    }

    pub fn poll_buttons(&mut self) -> nb::Result<[Option<Button>; 3], ()> {
        if matches!(self.read_state, ReadState::Started) {
            let val = match self.read_value_front() {
                Ok(val) => match val {
                    // Boundaries are midpoints between measured counts:
                    // Right 4, Left 1480, Confirm 2668, Back 3497, idle 4095.
                    0..742 => Some(Button::Right),
                    742..2074 => Some(Button::Left),
                    2074..3082 => Some(Button::Confirm),
                    3082..3796 => Some(Button::Back),
                    _ => None,
                },
                Err(nb::Error::WouldBlock) => return Err(nb::Error::WouldBlock),
                Err(nb::Error::Other(e)) => return Err(nb::Error::Other(e)),
            };
            self.read_state = ReadState::Front { val }
        };
        // Front has been read, read side
        let res = if let ReadState::Front { val } = self.read_state {
            let side = match self.read_value_side() {
                Ok(val) => match val {
                    // Midpoints for Down 4, Up 2221, and assumed idle 4095.
                    // Confirm the physical Up/Down mapping on the device.
                    0..1112 => Some(Button::Down),
                    1112..3158 => Some(Button::Up),
                    _ => None,
                },
                Err(nb::Error::WouldBlock) => return Err(nb::Error::WouldBlock),
                Err(nb::Error::Other(e)) => return Err(nb::Error::Other(e)),
            };
            let power = self.read_power().then(|| Button::Power);
            [val, side, power]
        } else {
            unreachable!("state moves in order")
        };
        self.read_state = ReadState::Started;
        Ok(res)
    }
}
