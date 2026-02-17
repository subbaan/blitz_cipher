use crossterm::style::Color;
use crate::renderer::Renderer;
use crate::camera::Camera;
use crate::stages::{EnemyType, UfoPattern};

#[derive(Clone, Copy)]
pub struct WorldPos {
    pub x: f64,
    pub y: f64,
}

// === Ship ===

pub struct Ship {
    pub pos: WorldPos,
    pub alive: bool,
    pub invincible_timer: f64,
    pub speed: f64,
}

impl Ship {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: WorldPos { x, y },
            alive: true,
            invincible_timer: 0.0,
            speed: 28.0,
        }
    }

    pub fn update(&mut self, dt: f64, up: bool, down: bool, left: bool, right: bool,
                  analog_x: f64, analog_y: f64,
                  camera_scroll_x: f64, viewport_w: usize, viewport_h: usize) {
        if !self.alive { return; }

        if self.invincible_timer > 0.0 {
            self.invincible_timer -= dt;
        }

        let move_speed = self.speed * dt;
        let vert_speed = move_speed * 0.65; // vertical feels slower / more deliberate
        if up { self.pos.y -= vert_speed * if analog_y != 0.0 { analog_y.abs() } else { 1.0 }; }
        if down { self.pos.y += vert_speed * if analog_y != 0.0 { analog_y.abs() } else { 1.0 }; }
        if left { self.pos.x -= move_speed * if analog_x != 0.0 { analog_x.abs() } else { 1.0 }; }
        if right { self.pos.x += move_speed * if analog_x != 0.0 { analog_x.abs() } else { 1.0 }; }

        // Clamp to left ~40% of screen (in world coords)
        let screen_left = camera_scroll_x;
        let screen_right = camera_scroll_x + viewport_w as f64 * 0.40;
        self.pos.x = self.pos.x.clamp(screen_left + 1.0, screen_right);
        self.pos.y = self.pos.y.clamp(1.0, viewport_h as f64 - 2.0);
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        if !self.alive { return; }

        let screen_x = (self.pos.x - camera.scroll_x) as isize;
        let screen_y = self.pos.y as usize;

        // Blink when invincible
        if self.invincible_timer > 0.0 {
            let blink = (self.invincible_timer * 10.0) as u32;
            if blink % 2 == 0 { return; }
        }

        let ship_color = Color::Rgb { r: 220, g: 220, b: 255 };
        let engine_color = Color::Rgb { r: 255, g: 160, b: 40 };
        let cockpit_color = Color::Rgb { r: 100, g: 200, b: 255 };

        // Ship sprite: multi-row
        //   /=>
        //  <===>
        //   \=>
        if screen_x >= 1 && screen_x + 4 < renderer.width as isize {
            let sx = screen_x as usize;
            if screen_y >= 1 && screen_y + 1 < renderer.height {
                // Top row
                renderer.set_fg_transparent(sx, screen_y - 1, '/', ship_color);
                renderer.set_fg_transparent(sx + 1, screen_y - 1, '=', ship_color);
                renderer.set_fg_transparent(sx + 2, screen_y - 1, '>', cockpit_color);

                // Middle row
                renderer.set_fg_transparent(sx.saturating_sub(1), screen_y, '<', engine_color);
                renderer.set_fg_transparent(sx, screen_y, '=', ship_color);
                renderer.set_fg_transparent(sx + 1, screen_y, '=', ship_color);
                renderer.set_fg_transparent(sx + 2, screen_y, '=', ship_color);
                renderer.set_fg_transparent(sx + 3, screen_y, '>', cockpit_color);

                // Bottom row
                renderer.set_fg_transparent(sx, screen_y + 1, '\\', ship_color);
                renderer.set_fg_transparent(sx + 1, screen_y + 1, '=', ship_color);
                renderer.set_fg_transparent(sx + 2, screen_y + 1, '>', cockpit_color);
            }
        }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        // (left, top, right, bottom)
        (self.pos.x - 1.0, self.pos.y - 1.0, self.pos.x + 4.0, self.pos.y + 2.0)
    }

    pub fn respawn(&mut self, camera_scroll_x: f64, viewport_h: usize) {
        self.pos.x = camera_scroll_x + 10.0;
        self.pos.y = viewport_h as f64 / 2.0;
        self.alive = true;
        self.invincible_timer = 3.0;
    }
}

// === Bullets ===

pub struct Bullet {
    pub pos: WorldPos,
    pub active: bool,
    pub speed: f64,
}

impl Bullet {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: WorldPos { x, y },
            active: true,
            speed: 60.0,
        }
    }

    pub fn update(&mut self, dt: f64, camera_scroll_x: f64, viewport_w: usize) {
        if !self.active { return; }
        self.pos.x += self.speed * dt;
        // Deactivate if off-screen
        if self.pos.x > camera_scroll_x + viewport_w as f64 + 5.0 {
            self.active = false;
        }
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        if !self.active { return; }
        let sx = (self.pos.x - camera.scroll_x) as isize;
        let sy = self.pos.y as usize;
        if sx >= 0 && (sx as usize) < renderer.width && sy < renderer.height {
            let color = Color::Rgb { r: 255, g: 255, b: 100 };
            renderer.set_fg_transparent(sx as usize, sy, '-', color);
        }
    }
}

// === Bombs ===

pub struct Bomb {
    pub pos: WorldPos,
    pub vel_x: f64,
    pub vel_y: f64,
    pub active: bool,
}

impl Bomb {
    pub fn new(x: f64, y: f64, forward_speed: f64) -> Self {
        Self {
            pos: WorldPos { x, y },
            vel_x: forward_speed * 0.5,
            vel_y: 0.0,
            active: true,
        }
    }

    pub fn update(&mut self, dt: f64, camera_scroll_x: f64, viewport_h: usize) {
        if !self.active { return; }
        self.vel_y += 40.0 * dt; // gravity
        self.pos.x += self.vel_x * dt;
        self.pos.y += self.vel_y * dt;

        // Off-screen check
        if self.pos.y > viewport_h as f64 + 2.0 || self.pos.x < camera_scroll_x - 5.0 {
            self.active = false;
        }
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        if !self.active { return; }
        let sx = (self.pos.x - camera.scroll_x) as isize;
        let sy = self.pos.y as usize;
        if sx >= 0 && (sx as usize) < renderer.width && sy < renderer.height {
            let color = Color::Rgb { r: 255, g: 100, b: 50 };
            renderer.set_fg_transparent(sx as usize, sy, 'o', color);
        }
    }
}

// === Enemies ===

#[derive(Clone)]
pub struct Enemy {
    pub pos: WorldPos,
    pub enemy_type: EnemyType,
    pub alive: bool,
    pub on_floor: bool,
    pub ufo_pattern: UfoPattern,
    pub phase: f64,      // for sine-wave movement
    pub launched: bool,   // for rockets
    pub vel_x: f64,       // for Meteor/Bouncer explicit velocity
    pub vel_y: f64,       // for launched rockets / Bouncer
    pub fire_timer: f64,  // for turrets
    pub retreating: bool, // for Charger retreat state
}

impl Enemy {
    pub fn new(x: f64, y: f64, enemy_type: EnemyType, on_floor: bool, ufo_pattern: UfoPattern, initial_phase: f64) -> Self {
        let (vel_x, vel_y) = match ufo_pattern {
            UfoPattern::Meteor => (-18.0, 12.0),
            UfoPattern::Bouncer => (-6.0, 10.0),
            _ => (0.0, 0.0),
        };
        Self {
            pos: WorldPos { x, y },
            enemy_type,
            alive: true,
            on_floor,
            ufo_pattern,
            phase: initial_phase,
            launched: false,
            vel_x,
            vel_y,
            fire_timer: 0.0,
            retreating: false,
        }
    }

    /// Returns true if a rocket just launched this frame.
    pub fn update(&mut self, dt: f64, ship_x: f64, ship_y: f64, viewport_h: f64) -> bool {
        if !self.alive { return false; }

        let mut just_launched = false;

        match self.enemy_type {
            EnemyType::Rocket => {
                if !self.launched {
                    // Launch when ship is nearby
                    let dx = ship_x - self.pos.x;
                    if dx.abs() < 30.0 && dx > -5.0 {
                        self.launched = true;
                        self.vel_y = -16.0;
                        just_launched = true;
                    }
                } else {
                    self.pos.y += self.vel_y * dt;
                    if self.pos.y < -5.0 {
                        self.alive = false;
                    }
                }
            }
            EnemyType::Ufo => {
                match self.ufo_pattern {
                    UfoPattern::SineWave => {
                        self.phase += dt * 3.0;
                        self.pos.y += self.phase.sin() * 8.0 * dt;
                        self.pos.x -= 5.0 * dt;
                    }
                    UfoPattern::Meteor => {
                        // Fast diagonal fall, top-right to bottom-left
                        self.pos.x += self.vel_x * dt;
                        self.pos.y += self.vel_y * dt;
                        // Self-destruct at bottom
                        if self.pos.y > viewport_h + 2.0 {
                            self.alive = false;
                        }
                    }
                    UfoPattern::Bouncer => {
                        // Drift left, bounce within the playable corridor
                        self.pos.x += self.vel_x * dt;
                        self.pos.y += self.vel_y * dt;
                        // Bounce within the middle ~70% of viewport (avoids terrain)
                        let bounce_top = viewport_h * 0.15;
                        let bounce_bot = viewport_h * 0.80;
                        if self.pos.y <= bounce_top {
                            self.pos.y = bounce_top;
                            self.vel_y = self.vel_y.abs();
                        } else if self.pos.y >= bounce_bot {
                            self.pos.y = bounce_bot;
                            self.vel_y = -self.vel_y.abs();
                        }
                    }
                    UfoPattern::ZSweep => {
                        // 6-segment cycle: downward-Z then mirrored upward-Z
                        // Down-Z: diag down-right, horiz left, diag down-left
                        // Up-Z:   diag up-right,   horiz left, diag up-left
                        self.phase += dt * 2.0;
                        let cycle = self.phase % 6.0;
                        let y_dir = if cycle < 3.0 { 1.0 } else { -1.0 };
                        let seg = cycle % 3.0;
                        if seg < 1.0 {
                            // Segment 1: diagonal right
                            self.pos.x += 8.0 * dt;
                            self.pos.y += 12.0 * y_dir * dt;
                        } else if seg < 2.0 {
                            // Segment 2: horizontal left
                            self.pos.x -= 14.0 * dt;
                        } else {
                            // Segment 3: diagonal left
                            self.pos.x -= 8.0 * dt;
                            self.pos.y += 12.0 * y_dir * dt;
                        }
                        // Overall leftward bias
                        self.pos.x -= 2.0 * dt;
                        // Clamp Y to playable area
                        self.pos.y = self.pos.y.clamp(viewport_h * 0.1, viewport_h * 0.85);
                    }
                    UfoPattern::Charger => {
                        // Rushes toward player's position at moment of charge.
                        // vel_y stores the locked target Y during attack.
                        let dx = ship_x - self.pos.x;

                        if self.retreating {
                            // Retreat: move left and drift toward center
                            let center_y = viewport_h * 0.5;
                            let drift = (center_y - self.pos.y).signum() * 5.0;
                            self.pos.x -= 10.0 * dt;
                            self.pos.y += drift * dt;
                            self.phase += dt;
                            if self.phase > 1.2 {
                                self.retreating = false;
                                self.phase = 0.0;
                            }
                        } else if self.vel_x > 0.0 {
                            // Charging — rush toward locked target Y
                            self.pos.x += self.vel_x * dt;
                            let target_dy = self.vel_y - self.pos.y;
                            // Move toward locked Y but don't track perfectly
                            self.pos.y += target_dy.signum() * 10.0 * dt;
                            self.phase += dt;
                            // Retreat after 0.8s or if passed player
                            if self.phase > 0.8 || self.pos.x > ship_x + 8.0 {
                                self.retreating = true;
                                self.vel_x = 0.0;
                                self.phase = 0.0;
                            }
                        } else if dx.abs() < 40.0 && dx > -8.0 {
                            // Trigger charge: snapshot player Y, begin rush
                            self.vel_y = ship_y; // lock target Y
                            self.vel_x = 30.0;   // charge speed
                            self.phase = 0.0;
                        } else {
                            // Idle: drift left with gentle sine
                            self.pos.x -= 4.0 * dt;
                            self.phase += dt * 2.0;
                            self.pos.y += self.phase.sin() * 4.0 * dt;
                        }
                    }
                    UfoPattern::FormationWave => {
                        // Group with large swooping arc + X wobble
                        self.phase += dt * 2.5;
                        self.pos.y += self.phase.sin() * 16.0 * dt;
                        self.pos.x -= 5.0 * dt;
                        self.pos.x += (self.phase * 1.5).cos() * 4.0 * dt;
                    }
                }
            }
            EnemyType::FuelTank => {
                // Stationary
            }
            EnemyType::Turret => {
                self.fire_timer += dt;
                // Turrets don't move, they fire (handled in game.rs)
            }
        }

        just_launched
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        if !self.alive { return; }
        let sx = (self.pos.x - camera.scroll_x) as isize;
        let sy = self.pos.y as usize;

        if sx < -3 || sx >= renderer.width as isize + 3 || sy >= renderer.height {
            return;
        }

        let sx = sx.max(0) as usize;

        match self.enemy_type {
            EnemyType::Rocket => {
                let color = Color::Rgb { r: 255, g: 80, b: 80 };
                if self.launched {
                    // Flying rocket
                    if sy > 0 { renderer.set_fg_transparent(sx, sy - 1, '^', color); }
                    renderer.set_fg_transparent(sx, sy, '|', color);
                    if sy + 1 < renderer.height {
                        renderer.set_fg_transparent(sx, sy + 1, '*', Color::Rgb { r: 255, g: 200, b: 50 });
                    }
                } else {
                    // Sitting on ground
                    if sy > 0 { renderer.set_fg_transparent(sx, sy - 1, '^', color); }
                    renderer.set_fg_transparent(sx, sy, 'A', color);
                }
            }
            EnemyType::Ufo => {
                match self.ufo_pattern {
                    UfoPattern::SineWave => {
                        // Classic purple saucer: (=)
                        let color = Color::Rgb { r: 200, g: 100, b: 255 };
                        if sx > 0 && sx + 2 < renderer.width {
                            renderer.set_fg_transparent(sx, sy, '(', color);
                            renderer.set_fg_transparent(sx + 1, sy, '=', Color::Rgb { r: 255, g: 255, b: 200 });
                            renderer.set_fg_transparent(sx + 2, sy, ')', color);
                        }
                    }
                    UfoPattern::Meteor => {
                        // Blazing orange/white streak: *@*
                        let color = Color::Rgb { r: 255, g: 160, b: 40 };
                        if sx > 0 && sx + 2 < renderer.width {
                            renderer.set_fg_transparent(sx, sy, '*', color);
                            renderer.set_fg_transparent(sx + 1, sy, '@', Color::Rgb { r: 255, g: 255, b: 220 });
                            renderer.set_fg_transparent(sx + 2, sy, '*', color);
                        }
                    }
                    UfoPattern::Bouncer => {
                        // Yellow bouncer: <O>
                        let color = Color::Rgb { r: 255, g: 230, b: 50 };
                        if sx > 0 && sx + 2 < renderer.width {
                            renderer.set_fg_transparent(sx, sy, '<', color);
                            renderer.set_fg_transparent(sx + 1, sy, 'O', Color::Rgb { r: 255, g: 255, b: 150 });
                            renderer.set_fg_transparent(sx + 2, sy, '>', color);
                        }
                    }
                    UfoPattern::ZSweep => {
                        // Green angular: <#>
                        let color = Color::Rgb { r: 50, g: 255, b: 100 };
                        if sx > 0 && sx + 2 < renderer.width {
                            renderer.set_fg_transparent(sx, sy, '<', color);
                            renderer.set_fg_transparent(sx + 1, sy, '#', Color::Rgb { r: 200, g: 255, b: 150 });
                            renderer.set_fg_transparent(sx + 2, sy, '>', color);
                        }
                    }
                    UfoPattern::Charger => {
                        // Scary red charger: {V}
                        let color = Color::Rgb { r: 255, g: 60, b: 40 };
                        if sx > 0 && sx + 2 < renderer.width {
                            renderer.set_fg_transparent(sx, sy, '{', color);
                            renderer.set_fg_transparent(sx + 1, sy, 'V', Color::Rgb { r: 255, g: 200, b: 60 });
                            renderer.set_fg_transparent(sx + 2, sy, '}', color);
                        }
                    }
                    UfoPattern::FormationWave => {
                        // Cyan formation: [~]
                        let color = Color::Rgb { r: 60, g: 200, b: 255 };
                        if sx > 0 && sx + 2 < renderer.width {
                            renderer.set_fg_transparent(sx, sy, '[', color);
                            renderer.set_fg_transparent(sx + 1, sy, '~', Color::Rgb { r: 180, g: 255, b: 255 });
                            renderer.set_fg_transparent(sx + 2, sy, ']', color);
                        }
                    }
                }
            }
            EnemyType::FuelTank => {
                let color = Color::Rgb { r: 255, g: 200, b: 50 };
                if sx + 2 < renderer.width {
                    renderer.set_fg_transparent(sx, sy, '[', color);
                    renderer.set_fg_transparent(sx + 1, sy, 'F', Color::Rgb { r: 255, g: 100, b: 50 });
                    renderer.set_fg_transparent(sx + 2, sy, ']', color);
                }
            }
            EnemyType::Turret => {
                let color = Color::Rgb { r: 180, g: 180, b: 200 };
                if sx + 1 < renderer.width {
                    renderer.set_fg_transparent(sx, sy, '{', color);
                    renderer.set_fg_transparent(sx + 1, sy, '}', color);
                    if sy + 1 < renderer.height {
                        renderer.set_fg_transparent(sx, sy + 1, 'v', Color::Rgb { r: 255, g: 50, b: 50 });
                    }
                }
            }
        }
    }

    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        match self.enemy_type {
            EnemyType::Rocket => (self.pos.x, self.pos.y - 1.0, self.pos.x + 1.0, self.pos.y + 1.0),
            EnemyType::Ufo => (self.pos.x, self.pos.y, self.pos.x + 3.0, self.pos.y + 1.0),
            EnemyType::FuelTank => (self.pos.x, self.pos.y, self.pos.x + 3.0, self.pos.y + 1.0),
            EnemyType::Turret => (self.pos.x, self.pos.y, self.pos.x + 2.0, self.pos.y + 2.0),
        }
    }

    pub fn score_value(&self) -> u32 {
        match self.enemy_type {
            EnemyType::Rocket => 50,
            EnemyType::Ufo => match self.ufo_pattern {
                UfoPattern::SineWave => 100,
                UfoPattern::Meteor => 80,
                UfoPattern::Bouncer => 100,
                UfoPattern::ZSweep => 120,
                UfoPattern::Charger => 200,
                UfoPattern::FormationWave => 150,
            },
            EnemyType::FuelTank => 150,
            EnemyType::Turret => 80,
        }
    }
}

// === Explosions (visual effect) ===

pub struct Explosion {
    pub pos: WorldPos,
    pub timer: f64,
    pub duration: f64,
}

impl Explosion {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: WorldPos { x, y },
            timer: 0.0,
            duration: 0.4,
        }
    }

    pub fn update(&mut self, dt: f64) -> bool {
        self.timer += dt;
        self.timer < self.duration
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        let sx = (self.pos.x - camera.scroll_x) as isize;
        let sy = self.pos.y as usize;

        if sx < 0 || sx >= renderer.width as isize || sy >= renderer.height {
            return;
        }

        let sx = sx as usize;
        let phase = (self.timer / self.duration * 3.0) as usize;

        let (chars, color) = match phase {
            0 => (['*', '+', '*'], Color::Rgb { r: 255, g: 255, b: 200 }),
            1 => (['#', '@', '#'], Color::Rgb { r: 255, g: 160, b: 50 }),
            _ => (['.', ':', '.'], Color::Rgb { r: 180, g: 80, b: 30 }),
        };

        if sx > 0 { renderer.set_fg_transparent(sx - 1, sy, chars[0], color); }
        renderer.set_fg_transparent(sx, sy, chars[1], color);
        if sx + 1 < renderer.width { renderer.set_fg_transparent(sx + 1, sy, chars[2], color); }
        if sy > 0 { renderer.set_fg_transparent(sx, sy - 1, chars[0], color); }
        if sy + 1 < renderer.height { renderer.set_fg_transparent(sx, sy + 1, chars[2], color); }
    }
}

// === Turret bullets ===

pub struct TurretBullet {
    pub pos: WorldPos,
    pub vel_y: f64,
    pub active: bool,
}

impl TurretBullet {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            pos: WorldPos { x, y },
            vel_y: 15.0, // downward
            active: true,
        }
    }

    pub fn update(&mut self, dt: f64, viewport_h: usize) {
        if !self.active { return; }
        self.pos.y += self.vel_y * dt;
        if self.pos.y > viewport_h as f64 + 2.0 || self.pos.y < -2.0 {
            self.active = false;
        }
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &Camera) {
        if !self.active { return; }
        let sx = (self.pos.x - camera.scroll_x) as isize;
        let sy = self.pos.y as usize;
        if sx >= 0 && (sx as usize) < renderer.width && sy < renderer.height {
            renderer.set_fg_transparent(sx as usize, sy, '|', Color::Rgb { r: 255, g: 80, b: 80 });
        }
    }
}
