use crossterm::event::KeyCode;

pub const BIND_LABELS: [&str; 8] = [
    "Up", "Down", "Left", "Right", "Fire", "Bomb", "Quit", "Restart",
];

pub struct KeyBindings {
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub fire: KeyCode,
    pub bomb: KeyCode,
    pub quit: KeyCode,
    pub restart: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            up: KeyCode::Up,
            down: KeyCode::Down,
            left: KeyCode::Left,
            right: KeyCode::Right,
            fire: KeyCode::Char('z'),
            bomb: KeyCode::Char('x'),
            quit: KeyCode::Char('q'),
            restart: KeyCode::Char('r'),
        }
    }
}

impl KeyBindings {
    /// Get binding by index (0..8) for config UI
    pub fn get(&self, index: usize) -> KeyCode {
        match index {
            0 => self.up,
            1 => self.down,
            2 => self.left,
            3 => self.right,
            4 => self.fire,
            5 => self.bomb,
            6 => self.quit,
            7 => self.restart,
            _ => KeyCode::Null,
        }
    }

    /// Set binding by index (0..8)
    pub fn set(&mut self, index: usize, key: KeyCode) {
        match index {
            0 => self.up = key,
            1 => self.down = key,
            2 => self.left = key,
            3 => self.right = key,
            4 => self.fire = key,
            5 => self.bomb = key,
            6 => self.quit = key,
            7 => self.restart = key,
            _ => {}
        }
    }
}

/// Serialize a KeyCode to a string for save files
pub fn keycode_to_string(key: KeyCode) -> String {
    match key {
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::Delete => "Delete".into(),
        KeyCode::Insert => "Insert".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => format!("Char:{}", c),
        KeyCode::F(n) => format!("F:{}", n),
        _ => "Unknown".into(),
    }
}

/// Deserialize a string back to a KeyCode
pub fn string_to_keycode(s: &str) -> Option<KeyCode> {
    match s {
        "Up" => Some(KeyCode::Up),
        "Down" => Some(KeyCode::Down),
        "Left" => Some(KeyCode::Left),
        "Right" => Some(KeyCode::Right),
        "Enter" => Some(KeyCode::Enter),
        "Esc" => Some(KeyCode::Esc),
        "Backspace" => Some(KeyCode::Backspace),
        "Tab" => Some(KeyCode::Tab),
        "Delete" => Some(KeyCode::Delete),
        "Insert" => Some(KeyCode::Insert),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "PageUp" => Some(KeyCode::PageUp),
        "PageDown" => Some(KeyCode::PageDown),
        "Space" => Some(KeyCode::Char(' ')),
        s if s.starts_with("Char:") => {
            s[5..].chars().next().map(KeyCode::Char)
        }
        s if s.starts_with("F:") => {
            s[2..].parse::<u8>().ok().map(KeyCode::F)
        }
        _ => None,
    }
}

/// Human-readable name for a KeyCode
pub fn keycode_name(key: KeyCode) -> String {
    match key {
        KeyCode::Up => "\u{2191}".into(),
        KeyCode::Down => "\u{2193}".into(),
        KeyCode::Left => "\u{2190}".into(),
        KeyCode::Right => "\u{2192}".into(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Backspace => "Bksp".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::Delete => "Del".into(),
        KeyCode::Insert => "Ins".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PgUp".into(),
        KeyCode::PageDown => "PgDn".into(),
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => c.to_uppercase().to_string(),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Null => "None".into(),
        _ => "???".into(),
    }
}
