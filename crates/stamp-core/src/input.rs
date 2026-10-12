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

const BUTTONS: [Button; 7] = [
    Button::Back,
    Button::Confirm,
    Button::Left,
    Button::Right,
    Button::Up,
    Button::Down,
    Button::Power,
];
const DEBOUNCE_MS: u64 = 20;

#[derive(Clone, Copy)]
struct ButtonState {
    candidate_pressed: bool,
    candidate_since_ms: u64,
    accepted_pressed: bool,
}

/// Accepts a button transition after its sampled state stays unchanged for 20 ms.
pub struct Debouncer {
    buttons: [ButtonState; 7],
}

impl Debouncer {
    /// Start with every button released. A button held at startup is debounced too.
    pub const fn new() -> Self {
        Self {
            buttons: [ButtonState {
                candidate_pressed: false,
                candidate_since_ms: 0,
                accepted_pressed: false,
            }; 7],
        }
    }

    /// Supply a complete snapshot and a monotonically increasing time in milliseconds.
    /// Call regularly (roughly every 5 ms); do not substitute an empty snapshot
    /// for an ADC reading that is still pending.
    ///
    /// Event slots correspond to Back, Confirm, Left, Right, Up, Down, and Power.
    /// Unchanged buttons return `None`; holding a button does not repeat events.
    pub fn update(
        &mut self,
        snapshot: [Option<Button>; 3],
        now_ms: u64,
    ) -> [Option<InputEvent>; 7] {
        let mut events = [None; 7];
        for (index, button) in BUTTONS.iter().copied().enumerate() {
            let pressed = snapshot.contains(&Some(button));
            let state = &mut self.buttons[index];

            if pressed != state.candidate_pressed {
                state.candidate_pressed = pressed;
                state.candidate_since_ms = now_ms;
            }

            if state.candidate_pressed != state.accepted_pressed
                && now_ms.saturating_sub(state.candidate_since_ms) >= DEBOUNCE_MS
            {
                state.accepted_pressed = state.candidate_pressed;
                events[index] = Some(if state.accepted_pressed {
                    InputEvent::Pressed(button)
                } else {
                    InputEvent::Released(button)
                });
            }
        }
        events
    }
}

impl Default for Debouncer {
    fn default() -> Self {
        Self::new()
    }
}

/// Non-blocking event source for the initial explicit event loop.
pub trait InputSource {
    type Error;

    fn poll_event(&mut self) -> Result<Option<InputEvent>, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: [Option<Button>; 3] = [None; 3];
    const NO_EVENTS: [Option<InputEvent>; 7] = [None; 7];

    #[test]
    fn press_and_release_bounce_restart_the_stability_timer() {
        let mut debouncer = Debouncer::new();
        let held = [Some(Button::Back), None, None];
        assert_eq!(debouncer.update(IDLE, 0), NO_EVENTS);
        assert_eq!(debouncer.update(held, 10), NO_EVENTS);
        assert_eq!(debouncer.update(IDLE, 15), NO_EVENTS);
        assert_eq!(debouncer.update(held, 18), NO_EVENTS);
        assert_eq!(debouncer.update(held, 37), NO_EVENTS);
        let mut expected = NO_EVENTS;
        expected[0] = Some(InputEvent::Pressed(Button::Back));
        assert_eq!(debouncer.update(held, 38), expected);
        assert_eq!(debouncer.update(held, 100), NO_EVENTS);

        assert_eq!(debouncer.update(IDLE, 110), NO_EVENTS);
        assert_eq!(debouncer.update(held, 115), NO_EVENTS);
        assert_eq!(debouncer.update(IDLE, 118), NO_EVENTS);
        assert_eq!(debouncer.update(IDLE, 137), NO_EVENTS);
        expected[0] = Some(InputEvent::Released(Button::Back));
        assert_eq!(debouncer.update(IDLE, 138), expected);
        assert_eq!(debouncer.update(IDLE, 200), NO_EVENTS);
    }

    #[test]
    fn buttons_have_independent_timers_and_can_emit_together() {
        let mut debouncer = Debouncer::new();
        let back = [Some(Button::Back), None, None];
        let all = [Some(Button::Back), Some(Button::Up), Some(Button::Power)];
        assert_eq!(debouncer.update(back, 0), NO_EVENTS);
        assert_eq!(debouncer.update(all, 5), NO_EVENTS);
        let mut expected = NO_EVENTS;
        expected[0] = Some(InputEvent::Pressed(Button::Back));
        assert_eq!(debouncer.update(all, 20), expected);
        expected = NO_EVENTS;
        expected[4] = Some(InputEvent::Pressed(Button::Up));
        expected[6] = Some(InputEvent::Pressed(Button::Power));
        assert_eq!(debouncer.update(all, 25), expected);

        // Switching within a ladder releases the old button and presses the new one.
        let changed = [Some(Button::Confirm), Some(Button::Up), Some(Button::Power)];
        assert_eq!(debouncer.update(changed, 30), NO_EVENTS);
        expected = NO_EVENTS;
        expected[0] = Some(InputEvent::Released(Button::Back));
        expected[1] = Some(InputEvent::Pressed(Button::Confirm));
        assert_eq!(debouncer.update(changed, 50), expected);
    }
}
