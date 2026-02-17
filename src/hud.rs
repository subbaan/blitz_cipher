use crossterm::{cursor, style, queue};
use std::io;

pub const VERSION: &str = "v0.1.24";

pub struct Hud {
    pub score: u32,
    pub hi_score: u32,
    pub stage: usize,
    pub fuel: f64,     // 0.0 to 1.0
    pub lives: u32,
    pub progress: f64, // 0.0 to 1.0, stage progress for NAV indicator
}

impl Hud {
    pub fn new() -> Self {
        Self {
            score: 0,
            hi_score: 0,
            stage: 1,
            fuel: 1.0,
            lives: 3,
            progress: 0.0,
        }
    }

    pub fn render(&self, stdout: &mut io::BufWriter<io::Stdout>, width: usize, height: usize) -> io::Result<()> {
        // Top row: SCORE  HI  STAGE  VERSION
        let top = format!(
            " SCORE: {:>6}  HI: {:>6}  STAGE {}  {}",
            self.score, self.hi_score, self.stage, VERSION
        );
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            style::SetForegroundColor(style::Color::Rgb { r: 200, g: 200, b: 200 }),
            style::SetBackgroundColor(style::Color::Rgb { r: 20, g: 20, b: 40 }),
        )?;
        // Pad to full width
        let padded_top = format!("{:<width$}", top, width = width);
        queue!(stdout, style::Print(padded_top))?;

        // Bottom row: FUEL gauge + lives
        let fuel_width = 16usize;
        let filled = (self.fuel * fuel_width as f64) as usize;
        let empty = fuel_width.saturating_sub(filled);

        let fuel_color = if self.fuel > 0.5 {
            style::Color::Rgb { r: 50, g: 220, b: 50 }
        } else if self.fuel > 0.25 {
            style::Color::Rgb { r: 220, g: 220, b: 50 }
        } else {
            style::Color::Rgb { r: 220, g: 50, b: 50 }
        };

        let bar_filled: String = "█".repeat(filled);
        let bar_empty: String = "░".repeat(empty);

        let hearts: String = (0..self.lives).map(|_| "♥ ").collect();

        queue!(
            stdout,
            cursor::MoveTo(0, (height + 1) as u16),
            style::SetBackgroundColor(style::Color::Rgb { r: 20, g: 20, b: 40 }),
            style::SetForegroundColor(style::Color::Rgb { r: 200, g: 200, b: 200 }),
            style::Print(" FUEL ["),
            style::SetForegroundColor(fuel_color),
            style::Print(&bar_filled),
            style::SetForegroundColor(style::Color::Rgb { r: 60, g: 60, b: 60 }),
            style::Print(&bar_empty),
            style::SetForegroundColor(style::Color::Rgb { r: 200, g: 200, b: 200 }),
            style::Print("] "),
            style::SetForegroundColor(style::Color::Rgb { r: 255, g: 80, b: 80 }),
            style::Print(&hearts),
        )?;

        // NAV progress indicator (right-aligned)
        // "NAV [▸▸▸▸▸▹▹▹▹▹]" = 4 + 1 + 10 + 1 + 1 = 17 chars
        let nav_segments = 10;
        let filled_nav = (self.progress * nav_segments as f64).round() as usize;
        let empty_nav = nav_segments - filled_nav;
        let nav_filled_str: String = "\u{25b8}".repeat(filled_nav);
        let nav_empty_str: String = "\u{25b9}".repeat(empty_nav);
        let nav_label = "NAV [";
        let nav_close = "]";
        let nav_total_len = nav_label.len() + nav_segments + nav_close.len() + 1; // +1 trailing space

        // Pad middle between hearts and NAV
        let used = 8 + fuel_width + 2 + self.lives as usize * 2;
        let right_edge = width.saturating_sub(nav_total_len);
        if right_edge > used {
            queue!(
                stdout,
                style::SetForegroundColor(style::Color::Rgb { r: 20, g: 20, b: 40 }),
                style::SetBackgroundColor(style::Color::Rgb { r: 20, g: 20, b: 40 }),
                style::Print(" ".repeat(right_edge - used)),
            )?;
        }

        queue!(
            stdout,
            style::SetBackgroundColor(style::Color::Rgb { r: 20, g: 20, b: 40 }),
            style::SetForegroundColor(style::Color::Rgb { r: 80, g: 200, b: 220 }),
            style::Print(nav_label),
            style::SetForegroundColor(style::Color::Rgb { r: 220, g: 220, b: 240 }),
            style::Print(&nav_filled_str),
            style::SetForegroundColor(style::Color::Rgb { r: 60, g: 60, b: 80 }),
            style::Print(&nav_empty_str),
            style::SetForegroundColor(style::Color::Rgb { r: 80, g: 200, b: 220 }),
            style::Print(nav_close),
            style::SetForegroundColor(style::Color::Rgb { r: 20, g: 20, b: 40 }),
            style::Print(" "),
        )?;

        Ok(())
    }
}
