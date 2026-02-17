pub struct Camera {
    pub scroll_x: f64,
    pub scroll_speed: f64, // columns per second
    pub viewport_w: usize,
    pub viewport_h: usize,
    /// Monotonically increasing scroll distance — never adjusted by terrain trims.
    /// Used for parallax layers and stage progress so they don't glitch on trim.
    pub total_scrolled: f64,
}

impl Camera {
    pub fn new(viewport_w: usize, viewport_h: usize) -> Self {
        Self {
            scroll_x: 0.0,
            scroll_speed: 15.0,
            viewport_w,
            viewport_h,
            total_scrolled: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64) {
        let dx = self.scroll_speed * dt;
        self.scroll_x += dx;
        self.total_scrolled += dx;
    }

    /// Advance scroll for non-gameplay contexts (e.g. title screen)
    pub fn advance(&mut self, dx: f64) {
        self.scroll_x += dx;
        self.total_scrolled += dx;
    }

    /// Terrain offset — uses trim-adjusted scroll_x (terrain columns are also trimmed)
    pub fn terrain_offset(&self) -> usize {
        self.scroll_x as usize
    }

    /// Get the column offset for a parallax layer — uses total_scrolled (immune to trims)
    pub fn layer_offset(&self, scroll_divisor: usize) -> usize {
        (self.total_scrolled as usize) / scroll_divisor.max(1)
    }

    /// Get half-pixel offset for half-block parallax layers
    pub fn layer_offset_half(&self, scroll_divisor: usize) -> usize {
        ((self.total_scrolled * 2.0) as usize) / scroll_divisor.max(1)
    }

    pub fn reset(&mut self) {
        self.scroll_x = 0.0;
        self.total_scrolled = 0.0;
    }
}
