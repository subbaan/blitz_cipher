use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

use crate::input::InputState;

const STICK_DEADZONE: f64 = 0.3;

pub struct GamepadHandler {
    gilrs: Option<Gilrs>,
    active_gamepad: Option<GamepadId>,
    stick_x: f64,
    stick_y: f64,
}

impl GamepadHandler {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().ok();
        let active_gamepad = gilrs.as_ref().and_then(|g| {
            g.gamepads().next().map(|(id, _)| id)
        });
        Self { gilrs, active_gamepad, stick_x: 0.0, stick_y: 0.0 }
    }

    /// Drain gamepad events and merge state into InputState (OR with keyboard).
    pub fn poll(&mut self, input: &mut InputState) {
        let gilrs = match self.gilrs.as_mut() {
            Some(g) => g,
            None => return,
        };

        // Drain event queue, track connections and axis values
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    if self.active_gamepad.is_none() {
                        self.active_gamepad = Some(event.id);
                    }
                }
                EventType::Disconnected => {
                    if self.active_gamepad == Some(event.id) {
                        self.active_gamepad = gilrs.gamepads().next().map(|(id, _)| id);
                    }
                }
                EventType::AxisChanged(Axis::LeftStickX, val, _) => {
                    if Some(event.id) == self.active_gamepad {
                        self.stick_x = val as f64;
                    }
                }
                EventType::AxisChanged(Axis::LeftStickY, val, _) => {
                    if Some(event.id) == self.active_gamepad {
                        self.stick_y = val as f64;
                    }
                }
                _ => {}
            }
        }

        let gp = match self.active_gamepad {
            Some(id) => gilrs.gamepad(id),
            None => return,
        };

        // D-pad buttons
        input.up |= gp.is_pressed(Button::DPadUp);
        input.down |= gp.is_pressed(Button::DPadDown);
        input.left |= gp.is_pressed(Button::DPadLeft);
        input.right |= gp.is_pressed(Button::DPadRight);

        // Left stick with deadzone (tracked from events)
        if self.stick_x < -STICK_DEADZONE {
            input.left = true;
            input.analog_x = self.stick_x;
        }
        if self.stick_x > STICK_DEADZONE {
            input.right = true;
            input.analog_x = self.stick_x;
        }
        if self.stick_y > STICK_DEADZONE {
            input.up = true;
            input.analog_y = self.stick_y;
        }
        if self.stick_y < -STICK_DEADZONE {
            input.down = true;
            input.analog_y = self.stick_y;
        }

        // Action buttons
        input.fire |= gp.is_pressed(Button::South);   // A
        input.bomb |= gp.is_pressed(Button::West);    // X
        if gp.is_pressed(Button::East) {               // B
            input.restart = true;
            input.any_key = true;
        }
        if gp.is_pressed(Button::Start) {
            input.config_toggle = true;
        }
        if gp.is_pressed(Button::Select) {
            input.title = true;
        }
    }
}
