//! Logical input contracts; raw GPIO and ADC values do not cross this boundary.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Button {
    Back,
    Confirm,
    Left,
    Right,
    Up,
    Down,
    Power,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEvent {
    Pressed(Button),
    Released(Button),
}

/// Non-blocking event source for the initial explicit event loop.
pub trait InputSource {
    type Error;

    fn poll_event(&mut self) -> Result<Option<InputEvent>, Self::Error>;
}
