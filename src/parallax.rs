use crossterm::style::Color;
use rand::Rng;

use crate::camera::Camera;
use crate::renderer::Renderer;

/// Star field layer — sparse period characters at various brightnesses
pub struct StarField {
    /// Column (terminal) x positions and row positions, plus color
    stars: Vec<(usize, usize, Color)>,
    pub world_width: usize, // total width in terminal columns (wraps)
    pub scroll_divisor: usize,
}

impl StarField {
    pub fn generate(world_width_cols: usize, height: usize, density: f64, seed: u64) -> Self {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut stars = Vec::new();

        let star_colors = [
            Color::Rgb { r: 60, g: 60, b: 80 },
            Color::Rgb { r: 80, g: 80, b: 110 },
            Color::Rgb { r: 100, g: 100, b: 140 },
            Color::Rgb { r: 140, g: 140, b: 180 },
            Color::Rgb { r: 180, g: 170, b: 140 }, // warm star
            Color::Rgb { r: 120, g: 140, b: 180 }, // blue star
        ];

        let count = (world_width_cols as f64 * height as f64 * density) as usize;
        for _ in 0..count {
            let x = rng.gen_range(0..world_width_cols);
            let y = rng.gen_range(0..height);
            let color = star_colors[rng.gen_range(0..star_colors.len())];
            stars.push((x, y, color));
        }

        Self {
            stars,
            world_width: world_width_cols,
            scroll_divisor: 12, // slow drift behind mountains
        }
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        let offset = camera.layer_offset(self.scroll_divisor);
        let vw = renderer.width;

        for &(x, y, color) in &self.stars {
            if y >= renderer.height {
                continue;
            }
            // Wrap and compute screen position
            let screen_x = if x >= offset % self.world_width {
                x - (offset % self.world_width)
            } else {
                x + self.world_width - (offset % self.world_width)
            };

            if screen_x < vw {
                // Skip stars where a mountain layer has already painted the background
                let hx = screen_x * 2;
                let idx_l = y * renderer.width * 2 + hx;
                let idx_r = idx_l + 1;
                if idx_l < renderer.hb.len() && idx_r < renderer.hb.len() {
                    if renderer.hb[idx_l].color != Color::Black || renderer.hb[idx_r].color != Color::Black {
                        continue;
                    }
                }
                renderer.set_fg_transparent(screen_x, y, '.', color);
            }
        }
    }
}

/// Mountain silhouette layer — height-map filled downward
pub struct MountainLayer {
    /// Height at each column (in rows from bottom)
    heights: Vec<usize>,
    pub color: Color,
    pub scroll_divisor: usize,
}

impl MountainLayer {
    pub fn generate(
        world_width: usize,
        max_height: usize,
        min_height: usize,
        color: Color,
        scroll_divisor: usize,
        seed: u64,
    ) -> Self {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut heights = vec![0usize; world_width];

        // Generate smooth mountain silhouette using midpoint displacement
        let mut h = rng.gen_range(min_height..=max_height) as f64;
        for i in 0..world_width {
            // Slow random walk
            h += rng.gen_range(-0.8..0.8);
            // Occasional peaks
            if rng.gen_range(0..100) < 3 {
                h += rng.gen_range(2.0..5.0);
            }
            h = h.clamp(min_height as f64, max_height as f64);
            heights[i] = h as usize;
        }

        // Smooth pass
        let mut smoothed = heights.clone();
        for i in 1..world_width - 1 {
            smoothed[i] = (heights[i - 1] + heights[i] * 2 + heights[i + 1]) / 4;
        }

        Self {
            heights: smoothed,
            color,
            scroll_divisor,
        }
    }

    /// Render as half-block silhouette (filled from bottom)
    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        let offset = camera.layer_offset_half(self.scroll_divisor);
        let vw = renderer.width * 2;
        let vh = renderer.height;
        let world_w = self.heights.len();

        for screen_hx in 0..vw {
            let world_col = (screen_hx + offset) % world_w;
            let h = self.heights[world_col];

            // Fill from bottom up to height
            for row in 0..h.min(vh) {
                let y = vh - 1 - row;
                // Only set if the pixel is still black (don't overwrite nearer layers)
                let idx = y * renderer.width * 2 + screen_hx;
                if idx < renderer.hb.len() && renderer.hb[idx].color == Color::Black {
                    renderer.set_half_pixel(screen_hx, y, self.color);
                }
            }
        }
    }
}
