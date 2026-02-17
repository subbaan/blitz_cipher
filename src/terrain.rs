use crossterm::style::Color;

use crate::camera::Camera;
use crate::renderer::Renderer;
use crate::stages::ZoneType;

#[derive(Clone)]
pub struct TerrainColumn {
    pub ceiling: usize,  // rows from top (0 = no ceiling)
    pub floor: usize,    // rows from bottom (0 = no floor)
    pub surface_char: char,
    pub color: Color,
    pub highlight: Color, // surface edge highlight
    pub surface_palette: [char; 3],  // mixed chars for body texture
    pub zone_type: ZoneType,         // drives edge decoration in render()
}

impl Default for TerrainColumn {
    fn default() -> Self {
        Self {
            ceiling: 0,
            floor: 3,
            surface_char: '▓',
            color: Color::Rgb { r: 40, g: 120, b: 40 },
            highlight: Color::Rgb { r: 80, g: 180, b: 80 },
            surface_palette: ['▓', '▒', '░'],
            zone_type: ZoneType::OpenSky,
        }
    }
}

pub struct Terrain {
    pub columns: Vec<TerrainColumn>,
}

impl Terrain {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
        }
    }

    /// Generate terrain from a list of zones
    pub fn generate_from_zones(zones: &[ZoneDef], viewport_h: usize) -> Self {
        let mut columns = Vec::new();

        for zone in zones {
            let zone_cols = Self::generate_zone(zone, viewport_h, columns.len());
            columns.extend(zone_cols);
        }

        Self { columns }
    }

    fn generate_zone(zone: &ZoneDef, viewport_h: usize, start_col: usize) -> Vec<TerrainColumn> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut cols = Vec::with_capacity(zone.length);

        let max_floor = viewport_h / 2;
        let max_ceil = viewport_h / 2;
        let min_corridor = 4usize; // minimum gap between ceiling and floor

        let palette = get_surface_palette(zone.zone_type);

        for i in 0..zone.length {
            let t = i as f64 / zone.length as f64;

            // Interpolate ceiling and floor ranges
            let ceil_base = lerp(zone.ceiling_start as f64, zone.ceiling_end as f64, t);
            let floor_base = lerp(zone.floor_start as f64, zone.floor_end as f64, t);

            // Sine-wave undulation: three overlapping frequencies for organic feel
            let wave1 = (i as f64 * 0.04).sin() * zone.undulation;
            let wave2 = (i as f64 * 0.09 + 2.0).sin() * zone.undulation * 0.6;
            let wave3 = (i as f64 * 0.17 + 5.0).sin() * zone.undulation * 0.3;
            let undulation = wave1 + wave2 + wave3;

            // Random noise perturbation
            let ceil_perturb = rng.gen_range(-zone.roughness..zone.roughness);
            let floor_perturb = rng.gen_range(-zone.roughness..zone.roughness);

            let mut ceiling = (ceil_base + undulation * 0.5 + ceil_perturb).max(0.0) as usize;
            let mut floor = (floor_base + undulation + floor_perturb).max(0.0) as usize;

            // Clamp
            ceiling = ceiling.min(max_ceil);
            floor = floor.min(max_floor);

            // Enforce minimum corridor
            if ceiling + floor + min_corridor > viewport_h {
                let excess = (ceiling + floor + min_corridor) - viewport_h;
                if ceiling > floor {
                    ceiling = ceiling.saturating_sub(excess);
                } else {
                    floor = floor.saturating_sub(excess);
                }
            }

            let surface_chars = ['▓', '█', '▒', '░', '#', '='];
            let sc = surface_chars[zone.surface_style % surface_chars.len()];

            cols.push(TerrainColumn {
                ceiling,
                floor,
                surface_char: sc,
                color: zone.color,
                highlight: zone.highlight,
                surface_palette: palette,
                zone_type: zone.zone_type,
            });
        }

        cols
    }

    /// Render terrain onto the renderer's foreground layer
    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        let offset = camera.terrain_offset(); // uses trim-adjusted scroll_x
        let vw = renderer.width;
        let vh = renderer.height;

        for screen_x in 0..vw {
            let world_col = screen_x + offset;
            if world_col >= self.columns.len() {
                continue;
            }

            let col = &self.columns[world_col];

            // Draw ceiling
            if col.ceiling > 0 {
                for y in 0..col.ceiling.min(vh) {
                    if y == col.ceiling - 1 {
                        // Surface edge — zone-themed decoration
                        let ch = get_edge_decoration(col.zone_type, true, world_col);
                        renderer.set_fg(screen_x, y, ch, col.highlight, darken(col.color));
                    } else {
                        // Body — textured palette with depth gradient
                        let depth = (col.ceiling - 1 - y) as f64; // distance from surface edge going up
                        let ch = col.surface_palette[(world_col + y) % 3];
                        let fg = tint_color(col.color, 0.85_f64.powf(depth));
                        renderer.set_fg(screen_x, y, ch, fg, darken(col.color));
                    }
                }
            }

            // Draw floor
            if col.floor > 0 {
                let floor_start = vh.saturating_sub(col.floor);
                for y in floor_start..vh {
                    if y == floor_start {
                        // Surface edge — zone-themed decoration
                        let ch = get_edge_decoration(col.zone_type, false, world_col);
                        renderer.set_fg(screen_x, y, ch, col.highlight, darken(col.color));
                    } else {
                        // Body — textured palette with depth gradient
                        let depth = (y - floor_start) as f64; // distance from surface edge going down
                        let ch = col.surface_palette[(world_col + y) % 3];
                        let fg = tint_color(col.color, 0.85_f64.powf(depth));
                        renderer.set_fg(screen_x, y, ch, fg, darken(col.color));
                    }
                }
            }
        }
    }

    /// Get ceiling and floor at a world column
    pub fn get_at(&self, world_col: usize) -> Option<&TerrainColumn> {
        self.columns.get(world_col)
    }

    /// Level the floor 1 column each side of `center_col` to match its floor height,
    /// creating a flat pad effect for ground-mounted objects.
    pub fn level_floor_around(&mut self, center_col: usize) {
        if let Some(floor_val) = self.columns.get(center_col).map(|c| c.floor) {
            if center_col > 0 {
                if let Some(col) = self.columns.get_mut(center_col - 1) {
                    col.floor = floor_val;
                }
            }
            if let Some(col) = self.columns.get_mut(center_col + 1) {
                col.floor = floor_val;
            }
        }
    }

    /// Append another terrain's columns to the end of this one
    pub fn append(&mut self, other: Terrain) {
        self.columns.extend(other.columns);
    }

    /// Remove the first `count` columns from the terrain
    pub fn trim_front(&mut self, count: usize) {
        if count >= self.columns.len() {
            self.columns.clear();
        } else {
            self.columns.drain(..count);
        }
    }

    pub fn total_width(&self) -> usize {
        self.columns.len()
    }
}

pub struct ZoneDef {
    pub zone_type: ZoneType,
    pub length: usize,
    pub ceiling_start: usize,
    pub ceiling_end: usize,
    pub floor_start: usize,
    pub floor_end: usize,
    pub color: Color,
    pub highlight: Color,
    pub surface_style: usize,
    pub undulation: f64,    // amplitude of sine waves (0.0 = flat, 3.0 = gentle, 8.0 = extreme)
    pub roughness: f64,     // amplitude of random noise on top
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn darken(color: Color) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: r / 3,
            g: g / 3,
            b: b / 3,
        },
        _ => Color::Black,
    }
}

/// Scale an RGB color by a factor (0.0 = black, 1.0 = unchanged)
fn tint_color(color: Color, factor: f64) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f64 * factor).clamp(0.0, 255.0) as u8,
            g: (g as f64 * factor).clamp(0.0, 255.0) as u8,
            b: (b as f64 * factor).clamp(0.0, 255.0) as u8,
        },
        other => other,
    }
}

/// Get the 3-char surface texture palette for a zone type
fn get_surface_palette(zone_type: ZoneType) -> [char; 3] {
    match zone_type {
        ZoneType::OpenSky       => ['▓', '▒', '░'],
        ZoneType::Mountains     => ['█', '▓', '#'],
        ZoneType::CaveEntrance  => ['▒', '░', '·'],
        ZoneType::NarrowCave    => ['░', '▒', ':'],
        ZoneType::FuelDepot     => ['█', '▓', '='],
        ZoneType::RocketBase | ZoneType::BossApproach => ['▓', '█', '#'],
    }
}

/// Get a themed edge decoration character for ceiling or floor surface edges
fn get_edge_decoration(zone_type: ZoneType, is_ceiling: bool, world_col: usize) -> char {
    let slot = world_col % 5;
    match zone_type {
        ZoneType::OpenSky => {
            if is_ceiling {
                '_'
            } else {
                match slot {
                    0 => '♣',
                    1 => '"',
                    2 => '↑',
                    3 => '‾',
                    _ => '‾',
                }
            }
        }
        ZoneType::Mountains => {
            if is_ceiling {
                '_'
            } else {
                match slot {
                    0 => '^',
                    1 => '∧',
                    2 => '‾',
                    3 => '^',
                    _ => '‾',
                }
            }
        }
        ZoneType::CaveEntrance | ZoneType::NarrowCave => {
            if is_ceiling {
                match slot {
                    0 => '▼',
                    1 => '∨',
                    2 => 'v',
                    3 => '_',
                    _ => '_',
                }
            } else {
                match slot {
                    0 => '▲',
                    1 => '∧',
                    2 => '^',
                    3 => '‾',
                    _ => '‾',
                }
            }
        }
        ZoneType::FuelDepot => {
            match slot {
                0 => '┃',
                1 => '║',
                2 => '│',
                3 if is_ceiling => '_',
                3 => '‾',
                _ if is_ceiling => '_',
                _ => '‾',
            }
        }
        ZoneType::RocketBase | ZoneType::BossApproach => {
            match slot {
                0 => '═',
                1 => '─',
                2 => '╌',
                3 if is_ceiling => '_',
                3 => '‾',
                _ if is_ceiling => '_',
                _ => '‾',
            }
        }
    }
}
