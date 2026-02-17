use crossterm::style::Color;
use std::io;

pub const HALF_BLOCK: char = '\u{258C}'; // ▌

#[derive(Clone, Copy, PartialEq)]
pub struct HalfPixel {
    pub color: Color,
}

impl Default for HalfPixel {
    fn default() -> Self {
        Self { color: Color::Black }
    }
}

/// A cell in the foreground overlay (ASCII art layer)
#[derive(Clone, Copy, PartialEq)]
pub struct FgCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub active: bool, // if true, this cell overrides the half-block background
}

impl Default for FgCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::White,
            bg: Color::Black,
            active: false,
        }
    }
}

/// Terminal cell after compositing — what actually gets written
#[derive(Clone, Copy, PartialEq)]
pub struct TermCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Default for TermCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::Black,
            bg: Color::Black,
        }
    }
}

pub struct Renderer {
    pub width: usize,  // terminal columns
    pub height: usize, // terminal rows (gameplay area, excluding HUD)
    /// Half-pixel framebuffer: width*2 x height
    pub hb: Vec<HalfPixel>,
    /// Foreground overlay: width x height
    pub fg: Vec<FgCell>,
    /// Composited output: width x height
    current: Vec<TermCell>,
    previous: Vec<TermCell>,
    first_frame: bool,
}

impl Renderer {
    pub fn new(width: usize, height: usize) -> Self {
        let hb_size = width * 2 * height;
        let cell_size = width * height;
        Self {
            width,
            height,
            hb: vec![HalfPixel::default(); hb_size],
            fg: vec![FgCell::default(); cell_size],
            current: vec![TermCell::default(); cell_size],
            previous: vec![TermCell { ch: '\x01', fg: Color::Reset, bg: Color::Reset }; cell_size],
            first_frame: true,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        let hb_size = width * 2 * height;
        let cell_size = width * height;
        self.hb = vec![HalfPixel::default(); hb_size];
        self.fg = vec![FgCell::default(); cell_size];
        self.current = vec![TermCell::default(); cell_size];
        self.previous = vec![TermCell { ch: '\x01', fg: Color::Reset, bg: Color::Reset }; cell_size];
        self.first_frame = true;
    }

    /// Force a full redraw on the next render (invalidates double-buffer)
    pub fn force_redraw(&mut self) {
        self.first_frame = true;
    }

    /// Clear both buffers for a new frame
    pub fn clear(&mut self) {
        for p in self.hb.iter_mut() {
            *p = HalfPixel::default();
        }
        for c in self.fg.iter_mut() {
            *c = FgCell::default();
        }
    }

    /// Set a half-pixel in the background buffer
    /// hx is in half-pixel coordinates (0..width*2), y in rows
    pub fn set_half_pixel(&mut self, hx: usize, y: usize, color: Color) {
        if hx < self.width * 2 && y < self.height {
            self.hb[y * self.width * 2 + hx] = HalfPixel { color };
        }
    }

    /// Set a foreground cell (ASCII art overlay)
    pub fn set_fg(&mut self, x: usize, y: usize, ch: char, fg: Color, bg: Color) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.fg[idx] = FgCell { ch, fg, bg, active: true };
        }
    }

    /// Set a foreground cell with transparent background (inherits from half-block layer)
    pub fn set_fg_transparent(&mut self, x: usize, y: usize, ch: char, fg: Color) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            // Compute what the half-block bg would be for this cell
            let hx = x * 2;
            let left = self.hb[y * self.width * 2 + hx].color;
            let right = self.hb[y * self.width * 2 + hx + 1].color;
            // Use the average/dominant — or just use left as background
            let bg = if left == right { left } else { left };
            self.fg[idx] = FgCell { ch, fg, bg, active: true };
        }
    }

    /// Composite half-block background + foreground overlay into terminal cells
    fn composite(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let fg_cell = &self.fg[idx];

                if fg_cell.active {
                    // Foreground overrides
                    self.current[idx] = TermCell {
                        ch: fg_cell.ch,
                        fg: fg_cell.fg,
                        bg: fg_cell.bg,
                    };
                } else {
                    // Half-block background
                    let hx = x * 2;
                    let left = self.hb[y * self.width * 2 + hx].color;
                    let right = self.hb[y * self.width * 2 + hx + 1].color;

                    if left == right {
                        self.current[idx] = TermCell {
                            ch: ' ',
                            fg: left,
                            bg: left,
                        };
                    } else {
                        self.current[idx] = TermCell {
                            ch: HALF_BLOCK,
                            fg: left,
                            bg: right,
                        };
                    }
                }
            }
        }
    }

    /// Render to terminal with double-buffering (only changed cells)
    pub fn render(&mut self, stdout: &mut io::BufWriter<io::Stdout>) -> io::Result<()> {
        use crossterm::{cursor, style, queue};

        self.composite();

        let force_all = self.first_frame;
        self.first_frame = false;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let cell = &self.current[idx];
                let prev = &self.previous[idx];

                if force_all || cell != prev {
                    queue!(
                        stdout,
                        cursor::MoveTo(x as u16, (y + 1) as u16), // +1 for HUD top row
                        style::SetForegroundColor(cell.fg),
                        style::SetBackgroundColor(cell.bg),
                        style::Print(cell.ch)
                    )?;
                }
            }
        }

        // Swap buffers
        std::mem::swap(&mut self.current, &mut self.previous);

        Ok(())
    }
}
