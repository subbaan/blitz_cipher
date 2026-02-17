use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::keybinds::KeyBindings;

#[derive(Default)]
pub struct InputState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub fire: bool,
    pub bomb: bool,
    pub quit: bool,
    pub restart: bool,
    pub title: bool,
    pub any_key: bool,
    pub config_toggle: bool,
    pub cheat_toggle: bool,
    pub last_raw_key: Option<KeyCode>,
    pub analog_x: f64,
    pub analog_y: f64,
    // Keyboard held-state (tracked via press/release)
    kb_up: bool,
    kb_down: bool,
    kb_left: bool,
    kb_right: bool,
}

impl InputState {
    pub fn clear_actions(&mut self) {
        self.fire = false;
        self.bomb = false;
        self.quit = false;
        self.restart = false;
        self.title = false;
        self.any_key = false;
        self.config_toggle = false;
        self.cheat_toggle = false;
        self.last_raw_key = None;
    }

    /// Poll all pending input events (non-blocking)
    pub fn poll(&mut self, bindings: &KeyBindings) {
        self.clear_actions();

        while event::poll(Duration::ZERO).unwrap_or(false) {
            if let Ok(Event::Key(KeyEvent { code, modifiers, kind, .. })) = event::read() {
                // Only handle Press events (ignore Release/Repeat for action keys)
                use crossterm::event::KeyEventKind;
                match kind {
                    KeyEventKind::Press => {
                        // Ignore modifier-only keys for any_key detection
                        let is_modifier = matches!(code,
                            KeyCode::Modifier(_)
                        );
                        if !is_modifier {
                            self.any_key = true;
                        }
                        self.last_raw_key = Some(code);

                        if code == bindings.up {
                            self.kb_up = true;
                        } else if code == bindings.down {
                            self.kb_down = true;
                        } else if code == bindings.left {
                            self.kb_left = true;
                        } else if code == bindings.right {
                            self.kb_right = true;
                        } else if code == bindings.fire || code == KeyCode::Char(' ') {
                            self.fire = true;
                        } else if code == bindings.bomb {
                            self.bomb = true;
                        } else if code == bindings.quit || code == KeyCode::Esc {
                            self.quit = true;
                        } else if code == bindings.restart {
                            self.restart = true;
                        } else if code == KeyCode::Char('t') {
                            self.title = true;
                        } else if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
                            self.quit = true;
                        } else if code == KeyCode::Char('c') {
                            self.config_toggle = true;
                        } else if code == KeyCode::Char('=') {
                            self.cheat_toggle = true;
                        }
                    }
                    KeyEventKind::Release => {
                        if code == bindings.up {
                            self.kb_up = false;
                        } else if code == bindings.down {
                            self.kb_down = false;
                        } else if code == bindings.left {
                            self.kb_left = false;
                        } else if code == bindings.right {
                            self.kb_right = false;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Rebuild directionals from keyboard held-state each frame.
        // Gamepad will OR into these after poll() returns.
        self.up = self.kb_up;
        self.down = self.kb_down;
        self.left = self.kb_left;
        self.right = self.kb_right;

        // Reset analog values so gamepad can set them fresh each frame
        self.analog_x = 0.0;
        self.analog_y = 0.0;
    }
}
