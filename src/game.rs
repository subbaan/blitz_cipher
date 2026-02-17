use crossterm::style::Color;
use std::io::{self, Write};

use crate::audio::{AudioSystem, SoundEffect};
use crate::camera::Camera;
use crate::collision;
use crate::hud::Hud;
use crate::input::InputState;
use crate::keybinds::{self, KeyBindings, BIND_LABELS};
use crate::objects::*;
use crate::parallax::{MountainLayer, StarField};
use crate::renderer::Renderer;
use crate::save::SaveData;
use crate::stages::{self, EnemyType};
use crate::terrain::Terrain;

#[derive(PartialEq)]
pub enum GamePhase {
    Title,
    Countdown, // 3-2-1-GO before gameplay
    Playing,
    Dying,   // brief death animation
    GameOver,
    Config,    // key binding configuration modal
    CheatMenu, // debug/cheat menu for testing
}

pub struct ConfigState {
    pub selected: usize,      // cursor row (0..8)
    pub binding: bool,        // waiting for keypress to bind
    pub blink_timer: f64,
    pub return_to_title: bool, // true = return to Title on close, false = Playing
}

pub struct GameOverState {
    pub timer: f64,
    pub entering_name: bool,
    pub name: [char; 3],
    pub name_pos: usize,
    pub rank: Option<usize>,
    pub name_input_cooldown: f64,
}

pub struct CheatState {
    pub selected: usize,      // cursor row
    pub start_stage: usize,   // 1-6
    pub invincible: bool,
    pub infinite_fuel: bool,
}

pub struct GameState {
    pub phase: GamePhase,
    pub camera: Camera,
    pub renderer: Renderer,
    pub hud: Hud,
    pub terrain: Terrain,
    pub ship: Ship,
    pub bullets: Vec<Bullet>,
    pub bombs: Vec<Bomb>,
    pub enemies: Vec<Enemy>,
    pub explosions: Vec<Explosion>,
    pub turret_bullets: Vec<TurretBullet>,
    pub stars: StarField,
    pub distant_mountains: MountainLayer,
    pub near_mountains: MountainLayer,
    pub current_stage: usize,
    pub stage_boundary: f64,         // total_scrolled value where current stage starts
    pub current_stage_len: usize,    // length of the current stage's terrain
    pub next_stage_len: usize,       // length of the next stage (set when preloaded)
    pub next_stage_loaded: bool,     // prevent loading next stage multiple times
    pub fire_cooldown: f64,
    pub bomb_cooldown: f64,
    pub death_timer: f64,
    pub term_width: usize,
    pub term_height: usize,
    pub keybinds: KeyBindings,
    pub config: ConfigState,
    pub title_timer: f64,
    pub title_last_show_logo: bool,
    pub countdown_timer: f64,
    pub countdown_beep_flags: [bool; 4],
    pub audio: AudioSystem,
    pub save_data: SaveData,
    pub game_over: GameOverState,
    pub cheat: CheatState,
}

impl GameState {
    pub fn new(term_width: usize, term_height: usize) -> Self {
        let viewport_h = term_height.saturating_sub(2); // -2 for HUD rows
        let viewport_w = term_width;

        let camera = Camera::new(viewport_w, viewport_h);

        let stars = StarField::generate(2000, viewport_h, 0.008, 42);
        let distant_mountains = MountainLayer::generate(
            2000, viewport_h / 3, 2,
            Color::Rgb { r: 25, g: 30, b: 50 },
            2, // ~0.5x — faster for smoother feel
            123,
        );
        let near_mountains = MountainLayer::generate(
            1500, viewport_h / 2, 3,
            Color::Rgb { r: 35, g: 45, b: 65 },
            1, // ~1x half-pixel rate — smooth near-layer motion
            456,
        );

        let terrain = Terrain::new();
        let ship = Ship::new(10.0, viewport_h as f64 / 2.0);
        let renderer = Renderer::new(viewport_w, viewport_h);
        let mut hud = Hud::new();

        let save_data = SaveData::load();
        let keybinds = KeyBindings {
            up: save_data.keybindings.up,
            down: save_data.keybindings.down,
            left: save_data.keybindings.left,
            right: save_data.keybindings.right,
            fire: save_data.keybindings.fire,
            bomb: save_data.keybindings.bomb,
            quit: save_data.keybindings.quit,
            restart: save_data.keybindings.restart,
        };
        hud.hi_score = save_data.top_score();

        Self {
            phase: GamePhase::Title,
            camera,
            renderer,
            hud,
            terrain,
            ship,
            bullets: Vec::new(),
            bombs: Vec::new(),
            enemies: Vec::new(),
            explosions: Vec::new(),
            turret_bullets: Vec::new(),
            stars,
            distant_mountains,
            near_mountains,
            current_stage: 1,
            stage_boundary: 0.0,
            current_stage_len: 0,
            next_stage_len: 0,
            next_stage_loaded: false,
            fire_cooldown: 0.0,
            bomb_cooldown: 0.0,
            death_timer: 0.0,
            term_width,
            term_height,
            keybinds,
            config: ConfigState { selected: 0, binding: false, blink_timer: 0.0, return_to_title: false },
            title_timer: 0.0,
            title_last_show_logo: true,
            countdown_timer: 0.0,
            countdown_beep_flags: [false; 4],
            audio: AudioSystem::new(),
            save_data,
            game_over: GameOverState {
                timer: 0.0,
                entering_name: false,
                name: ['A', 'A', 'A'],
                name_pos: 0,
                rank: None,
                name_input_cooldown: 0.0,
            },
            cheat: CheatState {
                selected: 0,
                start_stage: 1,
                invincible: false,
                infinite_fuel: false,
            },
        }
    }

    pub fn handle_resize(&mut self, width: usize, height: usize) {
        self.term_width = width;
        self.term_height = height;
        let vh = height.saturating_sub(2);
        self.camera.viewport_w = width;
        self.camera.viewport_h = vh;
        self.renderer.resize(width, vh);
    }

    pub fn return_to_title(&mut self) {
        self.phase = GamePhase::Title;
        self.title_timer = 0.0;
        self.title_last_show_logo = true;
        self.camera.reset();
        self.ship.respawn(0.0, self.camera.viewport_h);
        self.enemies.clear();
        self.bullets.clear();
        self.bombs.clear();
        self.turret_bullets.clear();
        self.explosions.clear();
        self.renderer.force_redraw();
    }

    pub fn start_game(&mut self) {
        self.phase = GamePhase::Countdown;
        self.countdown_timer = 0.0;
        self.countdown_beep_flags = [false; 4];
        self.current_stage = 1;
        self.hud.score = 0;
        self.hud.fuel = 1.0;
        self.hud.lives = 3;
        self.hud.stage = 1;
        self.hud.progress = 0.0;
        self.camera.reset();
        self.load_stage();
        self.ship.respawn(0.0, self.camera.viewport_h);
        self.ship.alive = true;
        self.renderer.force_redraw(); // clear intro panel artifacts
    }

    pub fn start_game_at_stage(&mut self, stage: usize) {
        self.phase = GamePhase::Countdown;
        self.countdown_timer = 0.0;
        self.countdown_beep_flags = [false; 4];
        self.current_stage = stage;
        self.hud.score = 0;
        self.hud.fuel = 1.0;
        self.hud.lives = 3;
        self.hud.stage = ((stage - 1) % 6) + 1;
        self.hud.progress = 0.0;
        self.camera.reset();
        self.load_stage();
        self.ship.respawn(0.0, self.camera.viewport_h);
        self.ship.alive = true;
        if self.cheat.invincible {
            self.ship.invincible_timer = f64::MAX;
        }
        self.renderer.force_redraw();
    }

    fn load_stage(&mut self) {
        let stage_def = stages::get_stage(self.current_stage, self.camera.viewport_h, self.camera.viewport_w);
        self.camera.scroll_speed = stage_def.scroll_speed;

        self.terrain = Terrain::generate_from_zones(&stage_def.zones, self.camera.viewport_h);
        self.stage_boundary = self.camera.total_scrolled;
        self.current_stage_len = self.terrain.total_width();
        self.next_stage_loaded = false;

        // Place enemies
        self.enemies.clear();
        self.bullets.clear();
        self.bombs.clear();
        self.turret_bullets.clear();
        self.explosions.clear();

        self.spawn_enemies_for_stage(&stage_def.enemies, 0);
    }

    fn load_next_stage_seamless(&mut self) {
        let next_stage_num = self.current_stage + 1;
        let stage_def = stages::get_stage(next_stage_num, self.camera.viewport_h, self.camera.viewport_w);

        let next_terrain = Terrain::generate_from_zones(&stage_def.zones, self.camera.viewport_h);
        let offset = self.terrain.total_width(); // enemies get offset by current total terrain length
        self.next_stage_len = next_terrain.total_width();

        self.terrain.append(next_terrain);
        self.spawn_enemies_for_stage(&stage_def.enemies, offset);
        self.next_stage_loaded = true;
    }

    fn spawn_enemies_for_stage(&mut self, placements: &[stages::EnemyPlacement], col_offset: usize) {
        use std::collections::HashMap;

        let vh = self.camera.viewport_h;

        // Track which floor columns are occupied and by which enemy index + type,
        // so we can detect rocket/fuel overlaps and favor fuel.
        let mut floor_occupied: HashMap<usize, (usize, EnemyType)> = HashMap::new();
        let first_new_idx = self.enemies.len();

        for placement in placements {
            let world_col = placement.col_offset + col_offset;

            if placement.on_floor {
                // Check for overlap with existing floor entities in nearby columns
                // Fuel tanks span 3 cols, rockets span 1 — check a range
                let check_range = 3usize;
                let mut overlap_found = false;
                for c in world_col.saturating_sub(check_range)..=(world_col + check_range) {
                    if let Some(&(existing_idx, existing_type)) = floor_occupied.get(&c) {
                        if placement.enemy_type == EnemyType::FuelTank && existing_type == EnemyType::Rocket {
                            // Fuel wins — remove the rocket
                            self.enemies[existing_idx].alive = false;
                            floor_occupied.remove(&c);
                        } else if placement.enemy_type == EnemyType::Rocket && existing_type == EnemyType::FuelTank {
                            // Fuel already there — skip this rocket
                            overlap_found = true;
                            break;
                        } else if c == world_col {
                            // Same type at exact same column — skip duplicate
                            overlap_found = true;
                            break;
                        }
                    }
                }
                if overlap_found {
                    continue;
                }

                let col = world_col.min(self.terrain.total_width().saturating_sub(1));
                let y = if let Some(tc) = self.terrain.get_at(col) {
                    (vh.saturating_sub(tc.floor).saturating_sub(1)) as f64
                } else {
                    (vh - 3) as f64
                };

                let idx = self.enemies.len();
                self.enemies.push(Enemy::new(
                    world_col as f64,
                    y,
                    placement.enemy_type,
                    placement.on_floor,
                    placement.ufo_pattern,
                    placement.initial_phase,
                ));
                floor_occupied.insert(world_col, (idx, placement.enemy_type));

                // Level terrain around floor-mounted rockets and fuel tanks
                if placement.enemy_type == EnemyType::Rocket || placement.enemy_type == EnemyType::FuelTank {
                    self.terrain.level_floor_around(col);
                }
            } else {
                let y = match placement.enemy_type {
                    EnemyType::Turret => {
                        let col = world_col.min(self.terrain.total_width().saturating_sub(1));
                        if let Some(tc) = self.terrain.get_at(col) {
                            tc.ceiling as f64
                        } else {
                            2.0
                        }
                    }
                    _ => {
                        if let Some(frac) = placement.spawn_y_override {
                            // Spawn at a specific viewport fraction (e.g. Meteors near top)
                            (vh as f64 * frac).max(1.0)
                        } else {
                            (vh as f64 * 0.3) + (placement.col_offset as f64 * 0.1).sin() * (vh as f64 * 0.2)
                        }
                    }
                };

                self.enemies.push(Enemy::new(
                    world_col as f64,
                    y,
                    placement.enemy_type,
                    placement.on_floor,
                    placement.ufo_pattern,
                    placement.initial_phase,
                ));
            }
        }

        // Clean out any enemies we marked dead due to overlap
        self.enemies.retain(|e| e.alive);
    }

    pub fn update(&mut self, dt: f64, input: &InputState) {
        match self.phase {
            GamePhase::Title => {
                self.title_timer += dt;
                self.camera.advance(14.0 * dt);
                self.ship.alive = false; // hide ship on title
                // Detect logo/scores cycle boundary and force redraw to clear remnants
                let show_logo = (self.title_timer % 16.0) < 8.0;
                if show_logo != self.title_last_show_logo {
                    self.title_last_show_logo = show_logo;
                    self.renderer.force_redraw();
                }
            }
            GamePhase::Countdown => {
                self.countdown_timer += dt;
                self.camera.advance(self.camera.scroll_speed * dt * 0.3); // slow drift

                // Sound triggers via threshold crossing
                let thresholds = [0.3, 1.3, 2.3, 3.3];
                for (i, &threshold) in thresholds.iter().enumerate() {
                    if !self.countdown_beep_flags[i] && self.countdown_timer >= threshold {
                        self.countdown_beep_flags[i] = true;
                        self.renderer.force_redraw(); // clear previous number
                        if i < 3 {
                            self.audio.play(SoundEffect::CountdownBeep);
                        } else {
                            self.audio.play(SoundEffect::StartJingle);
                        }
                    }
                }

                if self.countdown_timer >= 4.0 {
                    self.phase = GamePhase::Playing;
                    self.ship.invincible_timer = 2.0;
                    self.renderer.force_redraw(); // clear "GO!" overlay
                }
            }
            GamePhase::Playing => self.update_playing(dt, input),
            GamePhase::Dying => {
                self.death_timer -= dt;
                self.camera.advance(self.camera.scroll_speed * dt * 0.3); // slow scroll
                // Update explosions during death
                self.explosions.retain_mut(|e| e.update(dt));
                if self.death_timer <= 0.0 {
                    if self.hud.lives > 0 {
                        self.hud.lives -= 1;
                        self.hud.fuel = 1.0;
                        self.ship.respawn(self.camera.scroll_x, self.camera.viewport_h);
                        self.phase = GamePhase::Playing;
                    } else {
                        if self.hud.score > self.hud.hi_score {
                            self.hud.hi_score = self.hud.score;
                        }
                        let rank = self.save_data.qualifies(self.hud.score);
                        self.game_over = GameOverState {
                            timer: 0.0,
                            entering_name: rank.is_some(),
                            name: ['A', 'A', 'A'],
                            name_pos: 0,
                            rank,
                            name_input_cooldown: 0.0,
                        };
                        self.phase = GamePhase::GameOver;
                    }
                }
            }
            GamePhase::GameOver => {
                self.game_over.timer += dt;
                self.game_over.name_input_cooldown -= dt;
                if self.game_over.entering_name {
                    let mut handled = false;
                    if let Some(key) = input.last_raw_key {
                        handled = true;
                        match key {
                            crossterm::event::KeyCode::Up => {
                                let c = &mut self.game_over.name[self.game_over.name_pos];
                                *c = if *c == 'A' { 'Z' } else { ((*c as u8) - 1) as char };
                            }
                            crossterm::event::KeyCode::Down => {
                                let c = &mut self.game_over.name[self.game_over.name_pos];
                                *c = if *c == 'Z' { 'A' } else { ((*c as u8) + 1) as char };
                            }
                            crossterm::event::KeyCode::Left => {
                                if self.game_over.name_pos > 0 {
                                    self.game_over.name_pos -= 1;
                                }
                            }
                            crossterm::event::KeyCode::Right => {
                                if self.game_over.name_pos < 2 {
                                    self.game_over.name_pos += 1;
                                }
                            }
                            crossterm::event::KeyCode::Enter
                            | crossterm::event::KeyCode::Char('z')
                            | crossterm::event::KeyCode::Char(' ') => {
                                // Confirm name entry
                                let name: String = self.game_over.name.iter().collect();
                                if let Some(rank) = self.game_over.rank {
                                    self.save_data.insert(rank, name, self.hud.score, self.hud.stage);
                                    self.hud.hi_score = self.save_data.top_score();
                                    self.save_data.save();
                                }
                                self.game_over.rank = None;
                                self.game_over.entering_name = false;
                            }
                            _ => { handled = false; }
                        }
                    }
                    // Joystick/gamepad input with cooldown for repeat rate
                    if !handled && self.game_over.name_input_cooldown <= 0.0 {
                        if input.up {
                            let c = &mut self.game_over.name[self.game_over.name_pos];
                            *c = if *c == 'A' { 'Z' } else { ((*c as u8) - 1) as char };
                            self.game_over.name_input_cooldown = 0.15;
                        } else if input.down {
                            let c = &mut self.game_over.name[self.game_over.name_pos];
                            *c = if *c == 'Z' { 'A' } else { ((*c as u8) + 1) as char };
                            self.game_over.name_input_cooldown = 0.15;
                        } else if input.left {
                            if self.game_over.name_pos > 0 {
                                self.game_over.name_pos -= 1;
                            }
                            self.game_over.name_input_cooldown = 0.15;
                        } else if input.right {
                            if self.game_over.name_pos < 2 {
                                self.game_over.name_pos += 1;
                            }
                            self.game_over.name_input_cooldown = 0.15;
                        } else if input.fire {
                            let name: String = self.game_over.name.iter().collect();
                            if let Some(rank) = self.game_over.rank {
                                self.save_data.insert(rank, name, self.hud.score, self.hud.stage);
                                self.hud.hi_score = self.save_data.top_score();
                                self.save_data.save();
                            }
                            self.game_over.rank = None;
                            self.game_over.entering_name = false;
                            self.game_over.name_input_cooldown = 0.15;
                        }
                    }
                }
            }
            GamePhase::Config => {
                self.config.blink_timer += dt;
            }
            GamePhase::CheatMenu => {
                self.title_timer += dt;
                self.camera.advance(14.0 * dt);
            }
        }
    }

    fn update_playing(&mut self, dt: f64, input: &InputState) {
        // Camera
        self.camera.update(dt);

        // Seamless stage transition: when camera approaches end of current terrain,
        // generate next stage and append it
        let terrain_end = self.terrain.total_width();
        let camera_right = self.camera.scroll_x as usize + self.camera.viewport_w;
        if !self.next_stage_loaded && camera_right + self.camera.viewport_w >= terrain_end {
            self.load_next_stage_seamless();
        }

        // Detect stage boundary crossing: camera has scrolled past current stage's terrain
        let next_boundary = self.stage_boundary + self.current_stage_len as f64;
        if self.camera.total_scrolled >= next_boundary {
            self.current_stage += 1;
            self.hud.stage = ((self.current_stage - 1) % 6) + 1;
            self.stage_boundary = next_boundary;
            self.current_stage_len = self.next_stage_len;
            // Update scroll speed for the new stage
            let stage_def = stages::get_stage(self.current_stage, self.camera.viewport_h, self.camera.viewport_w);
            self.camera.scroll_speed = stage_def.scroll_speed;
            self.next_stage_loaded = false;
        }

        // Trim consumed columns to prevent unbounded growth
        // Only trim when old terrain is fully off-screen
        let trim_threshold = self.camera.viewport_w * 2;
        let scroll_col = self.camera.scroll_x as usize;
        if scroll_col > trim_threshold {
            let trim_count = scroll_col - self.camera.viewport_w;
            self.terrain.trim_front(trim_count);
            let trim_f = trim_count as f64;
            self.camera.scroll_x -= trim_f;
            self.ship.pos.x -= trim_f;
            for enemy in &mut self.enemies {
                enemy.pos.x -= trim_f;
            }
            for bullet in &mut self.bullets {
                bullet.pos.x -= trim_f;
            }
            for bomb in &mut self.bombs {
                bomb.pos.x -= trim_f;
            }
            for tb in &mut self.turret_bullets {
                tb.pos.x -= trim_f;
            }
            for explosion in &mut self.explosions {
                explosion.pos.x -= trim_f;
            }
        }

        // Ship
        self.ship.update(
            dt, input.up, input.down, input.left, input.right,
            input.analog_x, input.analog_y,
            self.camera.scroll_x, self.camera.viewport_w, self.camera.viewport_h,
        );

        // Fuel depletion
        if self.cheat.infinite_fuel {
            self.hud.fuel = 1.0;
        } else {
            self.hud.fuel -= 0.030 * dt; // 3% per second (~33s full tank)
            if self.hud.fuel <= 0.0 {
                self.hud.fuel = 0.0;
                self.kill_ship();
                return;
            }
        }

        // Maintain invincibility cheat
        if self.cheat.invincible {
            self.ship.invincible_timer = f64::MAX;
        }

        // Firing
        self.fire_cooldown = (self.fire_cooldown - dt).max(0.0);
        self.bomb_cooldown = (self.bomb_cooldown - dt).max(0.0);

        if input.fire && self.fire_cooldown <= 0.0 {
            self.bullets.push(Bullet::new(self.ship.pos.x + 4.0, self.ship.pos.y));
            self.fire_cooldown = 0.15;
            self.audio.play(SoundEffect::Shoot);
        }

        if input.bomb && self.bomb_cooldown <= 0.0 {
            self.bombs.push(Bomb::new(
                self.ship.pos.x + 2.0,
                self.ship.pos.y + 1.0,
                self.camera.scroll_speed,
            ));
            self.bomb_cooldown = 0.4;
            self.audio.play(SoundEffect::Bomb);
        }

        // Update projectiles
        for b in &mut self.bullets {
            b.update(dt, self.camera.scroll_x, self.camera.viewport_w);
        }
        for b in &mut self.bombs {
            b.update(dt, self.camera.scroll_x, self.camera.viewport_h);
        }
        for tb in &mut self.turret_bullets {
            tb.update(dt, self.camera.viewport_h);
        }

        // Update enemies
        let viewport_h = self.camera.viewport_h as f64;
        for enemy in &mut self.enemies {
            let just_launched = enemy.update(dt, self.ship.pos.x, self.ship.pos.y, viewport_h);
            if just_launched {
                self.audio.play(SoundEffect::RocketLaunch);
            }

            // Turret firing
            if enemy.enemy_type == EnemyType::Turret && enemy.alive && enemy.fire_timer > 2.0 {
                enemy.fire_timer = 0.0;
                self.turret_bullets.push(TurretBullet::new(enemy.pos.x + 0.5, enemy.pos.y + 2.0));
                self.audio.play(SoundEffect::TurretFire);
            }
        }

        // Update explosions
        self.explosions.retain_mut(|e| e.update(dt));

        // === Collisions ===

        // Bullets vs enemies
        let hits = collision::bullets_vs_enemies(&mut self.bullets, &mut self.enemies);
        for (idx, score) in &hits {
            self.hud.score += score;
            let enemy = &self.enemies[*idx];
            if enemy.enemy_type == EnemyType::FuelTank {
                self.hud.fuel = (self.hud.fuel + 0.3).min(1.0);
                self.audio.play(SoundEffect::FuelPickup);
            } else {
                self.audio.play(SoundEffect::Explosion);
            }
            self.explosions.push(Explosion::new(enemy.pos.x, enemy.pos.y));
        }

        // Bombs vs enemies
        let hits = collision::bombs_vs_enemies(&mut self.bombs, &mut self.enemies);
        for (idx, score) in &hits {
            self.hud.score += score;
            let enemy = &self.enemies[*idx];
            if enemy.enemy_type == EnemyType::FuelTank {
                self.hud.fuel = (self.hud.fuel + 0.3).min(1.0);
                self.audio.play(SoundEffect::FuelPickup);
            } else {
                self.audio.play(SoundEffect::Explosion);
            }
            self.explosions.push(Explosion::new(enemy.pos.x, enemy.pos.y));
        }

        // Bombs vs terrain
        let impacts = collision::bombs_vs_terrain(&mut self.bombs, &self.terrain, self.camera.viewport_h);
        for (x, y) in impacts {
            self.explosions.push(Explosion::new(x, y));
            self.audio.play(SoundEffect::Explosion);
        }

        // Ship vs terrain
        if collision::ship_vs_terrain(&self.ship, &self.terrain, self.camera.viewport_h) {
            self.kill_ship();
            return;
        }

        // Ship vs enemies
        if let Some(_) = collision::ship_vs_enemies(&self.ship, &self.enemies) {
            self.kill_ship();
            return;
        }

        // Turret bullets vs ship
        if collision::turret_bullets_vs_ship(&mut self.turret_bullets, &self.ship) {
            self.kill_ship();
            return;
        }

        // Cleanup inactive projectiles
        self.bullets.retain(|b| b.active);
        self.bombs.retain(|b| b.active);
        self.turret_bullets.retain(|b| b.active);

        // Update NAV progress
        if self.current_stage_len > 0 {
            let progress_in_stage = self.camera.total_scrolled - self.stage_boundary;
            self.hud.progress = (progress_in_stage / self.current_stage_len as f64).clamp(0.0, 1.0);
        }
    }

    fn kill_ship(&mut self) {
        self.explosions.push(Explosion::new(self.ship.pos.x, self.ship.pos.y));
        self.ship.alive = false;
        self.phase = GamePhase::Dying;
        self.death_timer = 1.5;
        self.audio.play(SoundEffect::PlayerDeath);
    }

    pub fn render(&mut self, stdout: &mut io::BufWriter<io::Stdout>) -> io::Result<()> {
        self.renderer.clear();

        // Render parallax backgrounds (half-block layers)
        // Mountains first so stars can check for overlap
        self.distant_mountains.render(&mut self.renderer, &self.camera);
        self.near_mountains.render(&mut self.renderer, &self.camera);
        self.stars.render(&mut self.renderer, &self.camera);

        // Render terrain (foreground ASCII layer)
        self.terrain.render(&mut self.renderer, &self.camera);

        // Render game objects (foreground)
        for enemy in &self.enemies {
            enemy.render(&mut self.renderer, &self.camera);
        }
        for bullet in &self.bullets {
            bullet.render(&mut self.renderer, &self.camera);
        }
        for bomb in &self.bombs {
            bomb.render(&mut self.renderer, &self.camera);
        }
        for tb in &self.turret_bullets {
            tb.render(&mut self.renderer, &self.camera);
        }
        for explosion in &self.explosions {
            explosion.render(&mut self.renderer, &self.camera);
        }
        self.ship.render(&mut self.renderer, &self.camera);

        // Output composited frame
        self.renderer.render(stdout)?;

        // HUD overlay
        self.hud.render(stdout, self.term_width, self.camera.viewport_h)?;

        // Phase-specific overlays
        match self.phase {
            GamePhase::Title => self.render_title(stdout)?,
            GamePhase::Countdown => self.render_countdown(stdout)?,
            GamePhase::GameOver => self.render_game_over(stdout)?,
            GamePhase::Config => self.render_config(stdout)?,
            GamePhase::CheatMenu => self.render_cheat_menu(stdout)?,
            _ => {}
        }

        stdout.flush()?;
        Ok(())
    }

    fn render_title(&self, stdout: &mut io::BufWriter<io::Stdout>) -> io::Result<()> {
        use crossterm::{cursor, style, queue};
        use crate::hud::VERSION;

        let cx = self.term_width / 2;
        let t = self.title_timer;
        let cycle = t % 16.0;
        let show_logo = cycle < 8.0;

        if show_logo {
            // === Logo phase (0–8s of each cycle) ===
            self.render_title_logo(stdout, cx, t)?;
        } else {
            // === Scores phase (8–16s of each cycle) ===
            self.render_title_scores(stdout, cx, t, cycle)?;
        }

        // --- Elements visible in both phases ---

        // "Press any key" — pulsing, appears after 1.8s
        let prompt_fade = ((t - 1.8) / 0.5).clamp(0.0, 1.0);
        if prompt_fade > 0.0 {
            let pulse = ((t * 3.0).sin() * 0.3 + 0.7).clamp(0.3, 1.0);
            let bright = (255.0 * prompt_fade * pulse) as u8;
            let prompt = "- Press any key to start -";
            let py = self.term_height * 3 / 4;
            let px = cx.saturating_sub(prompt.len() / 2);
            if py < self.term_height {
                queue!(
                    stdout,
                    cursor::MoveTo(px as u16, py as u16),
                    style::SetForegroundColor(Color::Rgb { r: bright, g: bright, b: (bright as u16 * 4 / 5) as u8 }),
                    style::SetBackgroundColor(Color::Reset),
                    style::Print(prompt),
                )?;
            }
        }

        // Subtle config hint
        let hint_fade = ((t - 1.0) / 0.8).clamp(0.0, 1.0);
        if hint_fade > 0.0 {
            let hint_y = self.term_height * 3 / 4 + 2;
            if hint_y < self.term_height {
                let hint = "C : Config";
                let hx = cx.saturating_sub(hint.len() / 2);
                let dim = (80.0 * hint_fade) as u8;
                queue!(
                    stdout,
                    cursor::MoveTo(hx as u16, hint_y as u16),
                    style::SetForegroundColor(Color::Rgb { r: dim, g: dim, b: (dim as u16 * 5 / 4).min(255) as u8 }),
                    style::SetBackgroundColor(Color::Reset),
                    style::Print(hint),
                )?;
            }
        }

        // Version in bottom-right
        let ver_y = self.term_height.saturating_sub(1);
        let ver_x = self.term_width.saturating_sub(VERSION.len() + 1);
        queue!(
            stdout,
            cursor::MoveTo(ver_x as u16, ver_y as u16),
            style::SetForegroundColor(Color::Rgb { r: 60, g: 60, b: 80 }),
            style::SetBackgroundColor(Color::Reset),
            style::Print(VERSION),
        )?;

        Ok(())
    }

    fn render_title_logo(&self, stdout: &mut io::BufWriter<io::Stdout>, cx: usize, t: f64) -> io::Result<()> {
        use crossterm::{cursor, style, queue};

        // Large ASCII art title with per-character color wave
        let logo: &[&str] = &[
            r"   ____  _     ___ _____ ____  ",
            r"  | __ )| |   |_ _|_   _|__  / ",
            r"  |  _ \| |    | |  | |   / /  ",
            r"  | |_) | |___ | |  | |  / /_  ",
            r"  |____/|_____|___| |_| /____| ",
            r"                              ",
            r"  ____ ___ ____  _   _ _____ ____  ",
            r" / ___|_ _|  _ \| | | | ____|  _ \ ",
            r"| |    | || |_) | |_| |  _| | |_) |",
            r"| |___ | ||  __/|  _  | |___|  _ < ",
            r" \____|___|_|   |_| |_|_____|_| \_\",
        ];

        let logo_w = logo.iter().map(|l| l.len()).max().unwrap_or(0);
        let logo_h = logo.len();
        let logo_x = cx.saturating_sub(logo_w / 2);
        // Place logo in upper third
        let logo_y = (self.term_height / 4).saturating_sub(logo_h / 2).max(1);

        // Fade in: first 1.5s the logo fades in (on first cycle), instant on subsequent
        let cycle = t % 16.0;
        let fade = if t < 16.0 { (t / 1.5).min(1.0) } else { (cycle / 0.5).min(1.0) };

        for (row, line) in logo.iter().enumerate() {
            let y = logo_y + row;
            if y >= self.term_height { break; }
            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' { continue; }
                let x = logo_x + col;
                if x >= self.term_width { break; }

                // Color wave: hue shifts across characters and time
                let wave = ((col as f64 * 0.15) + (row as f64 * 0.3) + t * 2.0).sin();
                let wave2 = ((col as f64 * 0.08) - t * 1.5).cos();

                let r = (120.0 + 80.0 * wave + 40.0 * wave2).clamp(0.0, 255.0) * fade;
                let g = (180.0 + 60.0 * wave2).clamp(0.0, 255.0) * fade;
                let b = (255.0 + 0.0 * wave).clamp(0.0, 255.0) * fade;

                queue!(
                    stdout,
                    cursor::MoveTo(x as u16, y as u16),
                    style::SetForegroundColor(Color::Rgb {
                        r: r as u8, g: g as u8, b: b as u8,
                    }),
                    style::SetBackgroundColor(Color::Reset),
                    style::Print(ch),
                )?;
            }
        }

        // Decorative divider line below logo
        let div_y = logo_y + logo_h + 1;
        if div_y < self.term_height {
            let div_w = 38.min(self.term_width - 2);
            let div_x = cx.saturating_sub(div_w / 2);
            let mut divider = String::new();
            for i in 0..div_w {
                let shimmer = ((i as f64 * 0.3 + t * 4.0).sin() * 0.5 + 0.5) > 0.5;
                divider.push(if shimmer { '=' } else { '-' });
            }
            let glow = ((t * 2.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
            queue!(
                stdout,
                cursor::MoveTo(div_x as u16, div_y as u16),
                style::SetForegroundColor(Color::Rgb {
                    r: (60.0 * glow) as u8,
                    g: (140.0 * glow) as u8,
                    b: (200.0 * glow) as u8,
                }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(&divider),
            )?;
        }

        // Enemy roster below divider (replaces old inline high scores)
        let roster_fade = if t < 16.0 { ((t - 2.0) / 0.8).clamp(0.0, 1.0) } else { (cycle / 0.5).min(1.0) };
        if roster_fade > 0.0 {
            let roster_y = div_y + 2;

            // Header with shimmer
            if roster_y < self.term_height {
                let header = "ENEMY  INTEL";
                let header_line = format!("═══ {} ═══", header);
                let hx = cx.saturating_sub(header_line.chars().count() / 2);
                let glow = ((t * 2.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0) * roster_fade;
                queue!(
                    stdout,
                    cursor::MoveTo(hx as u16, roster_y as u16),
                    style::SetForegroundColor(Color::Rgb {
                        r: (80.0 * glow) as u8,
                        g: (180.0 * glow) as u8,
                        b: (255.0 * glow) as u8,
                    }),
                    style::SetBackgroundColor(Color::Reset),
                    style::Print(&header_line),
                )?;
            }

            // Enemy data: (sprite, color_r, color_g, color_b, name, score)
            let enemies: &[(&str, u8, u8, u8, &str, u32)] = &[
                ("(=)", 200, 100, 255, "Scout",    100),
                ("*@*", 255, 160,  40, "Meteor",    80),
                ("<O>", 255, 230,  50, "Bouncer",  100),
                ("<#>",  50, 255, 100, "Z-Sweep",  120),
                ("{V}", 255,  60,  40, "Charger",  200),
                ("[~]",  60, 200, 255, "Wing",     150),
                ("^A^", 255,  80,  80, "Rocket",    50),
                ("{v}", 180, 180, 200, "Turret",    80),
                ("[F]", 255, 200,  50, "Fuel",     150),
            ];

            for (i, &(sprite, sr, sg, sb, name, score)) in enemies.iter().enumerate() {
                let row = roster_y + 1 + i;
                if row >= self.term_height { break; }

                // Build dot leader: "Name ········· score"
                // Fixed-width layout: " SPR  Name ········  NNN"
                let dots_total = 10usize.saturating_sub(name.len());
                let dots: String = "\u{00b7}".repeat(dots_total);
                let line_text = format!(" {}  {} {} {:>3}", sprite, name, dots, score);
                let lx = cx.saturating_sub(line_text.len() / 2);

                // Sprite in its game color
                let sprite_x = lx;
                queue!(
                    stdout,
                    cursor::MoveTo((sprite_x + 1) as u16, row as u16),
                    style::SetForegroundColor(Color::Rgb {
                        r: (sr as f64 * roster_fade) as u8,
                        g: (sg as f64 * roster_fade) as u8,
                        b: (sb as f64 * roster_fade) as u8,
                    }),
                    style::SetBackgroundColor(Color::Reset),
                    style::Print(sprite),
                )?;

                // Name in white
                let name_x = sprite_x + 6;
                queue!(
                    stdout,
                    cursor::MoveTo(name_x as u16, row as u16),
                    style::SetForegroundColor(Color::Rgb {
                        r: (220.0 * roster_fade) as u8,
                        g: (220.0 * roster_fade) as u8,
                        b: (230.0 * roster_fade) as u8,
                    }),
                    style::Print(name),
                )?;

                // Dot leader + score in dim
                let dots_x = name_x + name.len() + 1;
                let score_str = format!("{} {:>3}", dots, score);
                queue!(
                    stdout,
                    cursor::MoveTo(dots_x as u16, row as u16),
                    style::SetForegroundColor(Color::Rgb {
                        r: (100.0 * roster_fade) as u8,
                        g: (100.0 * roster_fade) as u8,
                        b: (120.0 * roster_fade) as u8,
                    }),
                    style::Print(&score_str),
                )?;
            }
        }

        Ok(())
    }

    fn render_title_scores(&self, stdout: &mut io::BufWriter<io::Stdout>, cx: usize, t: f64, cycle: f64) -> io::Result<()> {
        use crossterm::{cursor, style, queue};

        let phase_t = cycle - 8.0; // time within scores phase (0..8)
        let fade = (phase_t / 0.5).min(1.0);

        let panel_w = 38;
        // Total: header(1) + blank(1) + divider(1) + blank(1) + 10 entries + blank(1) + divider(1) = 16
        let total_h = 16;
        let py = (self.term_height / 2).saturating_sub(total_h / 2);
        let mut row = py;

        // "H I G H  S C O R E S" header with per-character wave
        if row < self.term_height {
            let header = "H I G H  S C O R E S";
            let hx = cx.saturating_sub(header.len() / 2);
            for (col, ch) in header.chars().enumerate() {
                if ch == ' ' {
                    queue!(stdout,
                        cursor::MoveTo((hx + col) as u16, row as u16),
                        style::SetBackgroundColor(Color::Reset),
                        style::Print(' '))?;
                    continue;
                }
                let wave = ((col as f64 * 0.3) + t * 2.5).sin();
                let wave2 = ((col as f64 * 0.15) - t * 1.8).cos();
                let r = (120.0 + 80.0 * wave + 40.0 * wave2).clamp(0.0, 255.0) * fade;
                let g = (180.0 + 60.0 * wave2).clamp(0.0, 255.0) * fade;
                let b = 255.0 * fade;
                queue!(stdout,
                    cursor::MoveTo((hx + col) as u16, row as u16),
                    style::SetForegroundColor(Color::Rgb { r: r as u8, g: g as u8, b: b as u8 }),
                    style::SetBackgroundColor(Color::Reset),
                    style::Print(ch))?;
            }
        }
        row += 2; // skip blank

        // Shimmer divider
        if row < self.term_height {
            let div_w = panel_w - 4;
            let dx = cx.saturating_sub(div_w / 2);
            let mut div = String::new();
            for i in 0..div_w {
                let shimmer = ((i as f64 * 0.3 + t * 4.0).sin() * 0.5 + 0.5) > 0.5;
                div.push(if shimmer { '═' } else { '─' });
            }
            let glow = ((t * 2.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0) * fade;
            queue!(stdout,
                cursor::MoveTo(dx as u16, row as u16),
                style::SetForegroundColor(Color::Rgb {
                    r: (80.0 * glow) as u8,
                    g: (180.0 * glow) as u8,
                    b: (255.0 * glow) as u8,
                }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(&div))?;
        }
        row += 2; // skip blank

        // 10 score entries
        for i in 0..10 {
            if row >= self.term_height { break; }

            let (name, score, stage) = if i < self.save_data.high_scores.len() {
                let e = &self.save_data.high_scores[i];
                (e.name.as_str(), e.score, e.stage)
            } else {
                ("---", 0, 0)
            };

            let line = format!("{:>2}. {}  {:>6}  STG {}",
                i + 1, name, score, stage);
            let lx = cx.saturating_sub(line.len() / 2);

            // Color gradient: gold (#1) → dim grey (#10)
            let gradient = 1.0 - (i as f64 / 9.0);
            let (r, g, b) = if score > 0 {
                (
                    (255.0 * gradient * fade) as u8,
                    ((200.0 * gradient + 55.0 * (1.0 - gradient)) * fade) as u8,
                    ((50.0 + 100.0 * (1.0 - gradient)) * fade) as u8,
                )
            } else {
                ((80.0 * fade) as u8, (80.0 * fade) as u8, (100.0 * fade) as u8)
            };

            queue!(stdout,
                cursor::MoveTo(lx as u16, row as u16),
                style::SetForegroundColor(Color::Rgb { r, g, b }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(&line))?;

            row += 1;
        }
        row += 1; // blank

        // Bottom divider
        if row < self.term_height {
            let div_w = panel_w - 4;
            let dx = cx.saturating_sub(div_w / 2);
            let mut div = String::new();
            for i in 0..div_w {
                let shimmer = ((i as f64 * 0.3 + t * 4.0).sin() * 0.5 + 0.5) > 0.5;
                div.push(if shimmer { '═' } else { '─' });
            }
            let glow = ((t * 2.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0) * fade;
            queue!(stdout,
                cursor::MoveTo(dx as u16, row as u16),
                style::SetForegroundColor(Color::Rgb {
                    r: (80.0 * glow) as u8,
                    g: (180.0 * glow) as u8,
                    b: (255.0 * glow) as u8,
                }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(&div))?;
        }

        Ok(())
    }

    fn render_countdown(&self, stdout: &mut io::BufWriter<io::Stdout>) -> io::Result<()> {
        use crossterm::{cursor, style, queue};

        let cx = self.term_width / 2;
        let cy = self.term_height / 2;
        let t = self.countdown_timer;

        // ASCII art numbers
        let art_3: &[&str] = &[
            " ████ ",
            "██  ██",
            "    ██",
            "  ██  ",
            "    ██",
            "██  ██",
            " ████ ",
        ];
        let art_2: &[&str] = &[
            " ████ ",
            "██  ██",
            "    ██",
            "  ██  ",
            " ██   ",
            "██    ",
            "██████",
        ];
        let art_1: &[&str] = &[
            "  ██  ",
            " ███  ",
            "  ██  ",
            "  ██  ",
            "  ██  ",
            "  ██  ",
            " ████ ",
        ];
        let art_go: &[&str] = &[
            " ██████  ██████  ██",
            "██       ██  ██  ██",
            "██  ███  ██  ██  ██",
            "██   ██  ██  ██    ",
            " ██████  ██████  ██",
        ];

        // Determine which number to show based on timer
        struct CountdownDisplay<'a> {
            art: &'a [&'a str],
            base_r: f64,
            base_g: f64,
            base_b: f64,
            appear_time: f64,
        }

        let displays = [
            CountdownDisplay { art: art_3, base_r: 255.0, base_g: 80.0, base_b: 40.0, appear_time: 0.3 },
            CountdownDisplay { art: art_2, base_r: 255.0, base_g: 200.0, base_b: 40.0, appear_time: 1.3 },
            CountdownDisplay { art: art_1, base_r: 80.0, base_g: 255.0, base_b: 80.0, appear_time: 2.3 },
            CountdownDisplay { art: art_go, base_r: 255.0, base_g: 255.0, base_b: 255.0, appear_time: 3.3 },
        ];

        // Find current display (last one whose appear_time has passed)
        let mut current_idx: Option<usize> = None;
        for (i, d) in displays.iter().enumerate() {
            if t >= d.appear_time {
                current_idx = Some(i);
            }
        }

        if let Some(idx) = current_idx {
            let display = &displays[idx];
            let elapsed = t - display.appear_time;

            // Punch-in scale effect: starts large, settles to normal
            let scale_factor = 1.0 + ((-elapsed * 8.0).exp() * 0.3);
            // Brightness: bright flash on appear, settles
            let flash = 1.0 + ((-elapsed * 6.0).exp() * 0.5);

            let art = display.art;
            let art_h = art.len();
            let art_w = art.iter().map(|l| l.len()).max().unwrap_or(0);

            let start_y = cy.saturating_sub(art_h / 2);
            let start_x = cx.saturating_sub(art_w / 2);

            for (row, line) in art.iter().enumerate() {
                let y = start_y + row;
                if y >= self.term_height { break; }

                for (col, ch) in line.chars().enumerate() {
                    if ch == ' ' { continue; }
                    let x = start_x + col;
                    if x >= self.term_width { break; }

                    // Per-character color with wave effect
                    let wave = ((col as f64 * 0.3 + row as f64 * 0.2 + t * 4.0).sin() * 0.15 + 1.0);

                    let r = (display.base_r * flash * wave * scale_factor).clamp(0.0, 255.0);
                    let g = (display.base_g * flash * wave * scale_factor).clamp(0.0, 255.0);
                    let b = (display.base_b * flash * wave * scale_factor).clamp(0.0, 255.0);

                    queue!(
                        stdout,
                        cursor::MoveTo(x as u16, y as u16),
                        style::SetForegroundColor(Color::Rgb {
                            r: r as u8, g: g as u8, b: b as u8,
                        }),
                        style::SetBackgroundColor(Color::Reset),
                        style::Print(ch),
                    )?;
                }
            }

            // For "GO!" — add color cycling wave effect
            if idx == 3 {
                // Already handled via wave above, but add extra shimmer
            }
        }

        Ok(())
    }

    fn render_game_over(&self, stdout: &mut io::BufWriter<io::Stdout>) -> io::Result<()> {
        use crossterm::{cursor, style, queue};

        let cx = self.term_width / 2;
        let t = self.game_over.timer;
        let panel_w = 38;
        let px = cx.saturating_sub(panel_w / 2);

        // Total lines: title(1) + blank(1) + score(1) + blank(1) + divider(1) + blank(1)
        //   + 10 scores + blank(1) + divider(1) + blank(1) + prompt(1) = 20
        let total_h = 20;
        let py = (self.term_height / 2).saturating_sub(total_h / 2);
        let bg = Color::Rgb { r: 10, g: 5, b: 15 };

        let mut row = py;

        // "G A M E   O V E R" — per-character color wave
        let title = "G A M E   O V E R";
        let tx = cx.saturating_sub(title.len() / 2);
        if row < self.term_height {
            for (col, ch) in title.chars().enumerate() {
                if ch == ' ' {
                    queue!(stdout,
                        cursor::MoveTo((tx + col) as u16, row as u16),
                        style::SetBackgroundColor(Color::Reset),
                        style::Print(' '))?;
                    continue;
                }
                let wave = ((col as f64 * 0.3) + t * 2.5).sin();
                let wave2 = ((col as f64 * 0.15) - t * 1.8).cos();
                let r = (200.0 + 55.0 * wave).clamp(0.0, 255.0);
                let g = (60.0 + 40.0 * wave2).clamp(0.0, 255.0);
                let b = (80.0 + 50.0 * wave).clamp(0.0, 255.0);
                queue!(stdout,
                    cursor::MoveTo((tx + col) as u16, row as u16),
                    style::SetForegroundColor(Color::Rgb { r: r as u8, g: g as u8, b: b as u8 }),
                    style::SetBackgroundColor(Color::Reset),
                    style::Print(ch))?;
            }
        }
        row += 2; // skip blank

        // "SCORE: xxxxx  STAGE: x"
        if row < self.term_height {
            let score_line = format!("SCORE: {:>6}   STAGE: {}", self.hud.score, self.hud.stage);
            let sx = cx.saturating_sub(score_line.len() / 2);
            queue!(stdout,
                cursor::MoveTo(sx as u16, row as u16),
                style::SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 220 }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(&score_line))?;
        }
        row += 2; // skip blank

        // Shimmer divider + "H I G H  S C O R E S" header
        if row < self.term_height {
            let header = "H I G H  S C O R E S";
            let div_pad = (panel_w.saturating_sub(header.len() + 4)) / 2;
            let mut div_line = String::new();
            for i in 0..div_pad {
                let shimmer = ((i as f64 * 0.3 + t * 4.0).sin() * 0.5 + 0.5) > 0.5;
                div_line.push(if shimmer { '═' } else { '─' });
            }
            div_line.push(' ');
            div_line.push_str(header);
            div_line.push(' ');
            for i in 0..div_pad {
                let shimmer = (((i + div_pad) as f64 * 0.3 + t * 4.0).sin() * 0.5 + 0.5) > 0.5;
                div_line.push(if shimmer { '═' } else { '─' });
            }
            let glow = ((t * 2.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
            let dx = cx.saturating_sub(div_line.chars().count() / 2);
            queue!(stdout,
                cursor::MoveTo(dx as u16, row as u16),
                style::SetForegroundColor(Color::Rgb {
                    r: (80.0 * glow) as u8,
                    g: (180.0 * glow) as u8,
                    b: (255.0 * glow) as u8,
                }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(&div_line))?;
        }
        row += 2; // skip blank

        // Score rows (10 entries)
        // When a new score is pending insertion, shift existing entries down visually
        let pending_rank = self.game_over.rank;
        for i in 0..10 {
            if row >= self.term_height { break; }

            let is_new = pending_rank == Some(i);
            let (name, score, stage) = if is_new {
                // Show the player's new score at this rank
                let entered_name: String = self.game_over.name.iter().collect();
                (entered_name, self.hud.score, self.hud.stage)
            } else {
                // Shift existing entries down by 1 if below the new entry
                let src_idx = match pending_rank {
                    Some(rank) if i > rank => i - 1,
                    _ => i,
                };
                if src_idx < self.save_data.high_scores.len() {
                    let e = &self.save_data.high_scores[src_idx];
                    (e.name.clone(), e.score, e.stage)
                } else {
                    ("---".to_string(), 0, 0)
                }
            };

            let marker = if is_new { " \u{25c4}NEW" } else { "" };
            let line = format!("{:>2}. {}  {:>6}  STG {}{}",
                i + 1, name, score, stage, marker);
            let lx = cx.saturating_sub(panel_w / 2) + 2;

            // Color gradient: gold (#1) → dim grey (#10)
            let gradient = 1.0 - (i as f64 / 9.0);
            let (r, g, b) = if is_new {
                // Blinking highlight for new entry
                let blink = ((t * 5.0).sin() * 0.4 + 0.6).clamp(0.2, 1.0);
                ((255.0 * blink) as u8, (255.0 * blink) as u8, (100.0 * blink) as u8)
            } else if score > 0 {
                (
                    (255.0 * gradient) as u8,
                    (200.0 * gradient + 55.0 * (1.0 - gradient)) as u8,
                    (50.0 + 100.0 * (1.0 - gradient)) as u8,
                )
            } else {
                (80, 80, 100)
            };

            // During name entry, render the name chars specially
            if is_new && self.game_over.entering_name {
                let prefix = format!("{:>2}. ", i + 1);
                queue!(stdout,
                    cursor::MoveTo(lx as u16, row as u16),
                    style::SetForegroundColor(Color::Rgb { r, g, b }),
                    style::SetBackgroundColor(bg),
                    style::Print(&prefix))?;

                // Render 3 name chars with cursor highlight
                for ci in 0..3 {
                    let ch = self.game_over.name[ci];
                    let is_cursor = ci == self.game_over.name_pos;
                    if is_cursor {
                        let blink = ((t * 6.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
                        queue!(stdout,
                            style::SetForegroundColor(Color::Rgb {
                                r: (100.0 * blink) as u8,
                                g: (255.0 * blink) as u8,
                                b: (255.0 * blink) as u8,
                            }),
                            style::SetBackgroundColor(Color::Rgb { r: 40, g: 40, b: 80 }),
                            style::Print(ch))?;
                    } else {
                        queue!(stdout,
                            style::SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 100 }),
                            style::SetBackgroundColor(bg),
                            style::Print(ch))?;
                    }
                }

                let suffix = format!("  {:>6}  STG {}{}", score, stage, marker);
                queue!(stdout,
                    style::SetForegroundColor(Color::Rgb { r, g, b }),
                    style::SetBackgroundColor(bg),
                    style::Print(&suffix))?;
            } else {
                queue!(stdout,
                    cursor::MoveTo(lx as u16, row as u16),
                    style::SetForegroundColor(Color::Rgb { r, g, b }),
                    style::SetBackgroundColor(bg),
                    style::Print(&line))?;
            }

            row += 1;
        }
        row += 1; // blank

        // Bottom divider
        if row < self.term_height {
            let mut div = String::new();
            let dw = panel_w - 4;
            for i in 0..dw {
                let shimmer = ((i as f64 * 0.3 + t * 4.0).sin() * 0.5 + 0.5) > 0.5;
                div.push(if shimmer { '═' } else { '─' });
            }
            let glow = ((t * 2.0).sin() * 0.3 + 0.7).clamp(0.4, 1.0);
            let dx = cx.saturating_sub(dw / 2);
            queue!(stdout,
                cursor::MoveTo(dx as u16, row as u16),
                style::SetForegroundColor(Color::Rgb {
                    r: (80.0 * glow) as u8,
                    g: (180.0 * glow) as u8,
                    b: (255.0 * glow) as u8,
                }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(&div))?;
        }
        row += 2; // blank

        // Prompt line
        if row < self.term_height {
            let prompt = if self.game_over.entering_name {
                "\u{2191}\u{2193}:Letter  \u{2190}\u{2192}:Move  Fire:Confirm"
            } else {
                "Press any key for title"
            };
            // Clear the full line width to erase any previous longer prompt
            let clear_x = cx.saturating_sub(panel_w / 2);
            queue!(stdout,
                cursor::MoveTo(clear_x as u16, row as u16),
                style::SetForegroundColor(Color::Reset),
                style::SetBackgroundColor(Color::Reset),
                style::Print(" ".repeat(panel_w)))?;
            let pulse = ((t * 3.0).sin() * 0.3 + 0.7).clamp(0.3, 1.0);
            let bright = (255.0 * pulse) as u8;
            let ppx = cx.saturating_sub(prompt.len() / 2);
            queue!(stdout,
                cursor::MoveTo(ppx as u16, row as u16),
                style::SetForegroundColor(Color::Rgb { r: bright, g: bright, b: (bright as u16 * 4 / 5) as u8 }),
                style::SetBackgroundColor(Color::Reset),
                style::Print(prompt))?;
        }

        Ok(())
    }

    fn render_config(&self, stdout: &mut io::BufWriter<io::Stdout>) -> io::Result<()> {
        use crossterm::{cursor, style, queue};

        let cx = self.term_width / 2;
        let cy = self.term_height / 2;

        let title_color = Color::Rgb { r: 100, g: 200, b: 255 };
        let normal_color = Color::Rgb { r: 180, g: 180, b: 200 };
        let selected_color = Color::Rgb { r: 255, g: 255, b: 100 };
        let binding_color = Color::Rgb { r: 255, g: 200, b: 50 };
        let dim_color = Color::Rgb { r: 100, g: 100, b: 120 };
        let bg = Color::Rgb { r: 10, g: 10, b: 30 };
        let sel_bg = Color::Rgb { r: 30, g: 30, b: 60 };

        let box_w = 30;
        // title + blank + 8 bindings + blank + help + close + bottom border = 14 lines
        let box_h = 14;
        let start_y = cy.saturating_sub(box_h / 2);
        let start_x = cx.saturating_sub(box_w / 2);

        // Top border
        let top = format!("\u{2554}{}\u{2557}", "\u{2550}".repeat(box_w - 2));
        queue!(stdout, cursor::MoveTo(start_x as u16, start_y as u16),
            style::SetForegroundColor(normal_color), style::SetBackgroundColor(bg),
            style::Print(&top))?;

        // Title
        let title_line = format!("\u{2551}{:^width$}\u{2551}", "KEY BINDINGS", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 1) as u16),
            style::SetForegroundColor(title_color), style::SetBackgroundColor(bg),
            style::Print(&title_line))?;

        // Blank
        let blank = format!("\u{2551}{:^width$}\u{2551}", "", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 2) as u16),
            style::SetForegroundColor(normal_color), style::SetBackgroundColor(bg),
            style::Print(&blank))?;

        // Binding rows
        for i in 0..8 {
            let y = start_y + 3 + i;
            let is_selected = i == self.config.selected;
            let key_name = if is_selected && self.config.binding {
                // Flashing "Press a key..." text
                if (self.config.blink_timer * 3.0) as u32 % 2 == 0 {
                    "[Press a key...]".into()
                } else {
                    "              ".into()
                }
            } else {
                keybinds::keycode_name(self.keybinds.get(i))
            };

            let prefix = if is_selected { ">" } else { " " };
            let content = format!(" {} {:<10}: {:<14}", prefix, BIND_LABELS[i], key_name);
            let line = format!("\u{2551}{:<width$}\u{2551}", content, width = box_w - 2);

            let (fg, row_bg) = if is_selected && self.config.binding {
                (binding_color, sel_bg)
            } else if is_selected {
                (selected_color, sel_bg)
            } else {
                (normal_color, bg)
            };

            queue!(stdout, cursor::MoveTo(start_x as u16, y as u16),
                style::SetForegroundColor(fg), style::SetBackgroundColor(row_bg),
                style::Print(&line))?;
        }

        // Blank
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 11) as u16),
            style::SetForegroundColor(normal_color), style::SetBackgroundColor(bg),
            style::Print(&blank))?;

        // Help line
        let help = format!("\u{2551}{:^width$}\u{2551}", "\u{2191}\u{2193}:Select  Enter:Bind", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 12) as u16),
            style::SetForegroundColor(dim_color), style::SetBackgroundColor(bg),
            style::Print(&help))?;

        // Close line
        let close = format!("\u{2551}{:^width$}\u{2551}", "Esc:Close", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 13) as u16),
            style::SetForegroundColor(dim_color), style::SetBackgroundColor(bg),
            style::Print(&close))?;

        // Bottom border
        let bottom = format!("\u{255a}{}\u{255d}", "\u{2550}".repeat(box_w - 2));
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 14) as u16),
            style::SetForegroundColor(normal_color), style::SetBackgroundColor(bg),
            style::Print(&bottom))?;

        Ok(())
    }

    fn render_cheat_menu(&self, stdout: &mut io::BufWriter<io::Stdout>) -> io::Result<()> {
        use crossterm::{cursor, style, queue};

        let cx = self.term_width / 2;
        let cy = self.term_height / 2;

        let title_color = Color::Rgb { r: 255, g: 100, b: 100 };
        let normal_color = Color::Rgb { r: 180, g: 180, b: 200 };
        let selected_color = Color::Rgb { r: 255, g: 255, b: 100 };
        let on_color = Color::Rgb { r: 100, g: 255, b: 100 };
        let off_color = Color::Rgb { r: 120, g: 120, b: 140 };
        let bg = Color::Rgb { r: 10, g: 10, b: 30 };
        let sel_bg = Color::Rgb { r: 30, g: 30, b: 60 };

        let box_w = 32;
        // title + blank + stage + invincible + inf_fuel + blank + help + start + bottom = 11
        let box_h = 11;
        let start_y = cy.saturating_sub(box_h / 2);
        let start_x = cx.saturating_sub(box_w / 2);

        // Top border
        let top = format!("\u{2554}{}\u{2557}", "\u{2550}".repeat(box_w - 2));
        queue!(stdout, cursor::MoveTo(start_x as u16, start_y as u16),
            style::SetForegroundColor(title_color), style::SetBackgroundColor(bg),
            style::Print(&top))?;

        // Title
        let title_line = format!("\u{2551}{:^width$}\u{2551}", "DEBUG MENU", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 1) as u16),
            style::SetForegroundColor(title_color), style::SetBackgroundColor(bg),
            style::Print(&title_line))?;

        // Blank
        let blank = format!("\u{2551}{:^width$}\u{2551}", "", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 2) as u16),
            style::SetForegroundColor(normal_color), style::SetBackgroundColor(bg),
            style::Print(&blank))?;

        // Menu items
        let items: Vec<(String, bool)> = vec![
            (format!("Start Stage:  < {} >", self.cheat.start_stage), false),
            (format!("Invincible:   {}", if self.cheat.invincible { "ON" } else { "OFF" }), self.cheat.invincible),
            (format!("Infinite Fuel:{}", if self.cheat.infinite_fuel { " ON" } else { " OFF" }), self.cheat.infinite_fuel),
            (">>> LAUNCH <<<".to_string(), false),
        ];

        for (i, (label, is_on)) in items.iter().enumerate() {
            let y = start_y + 3 + i;
            let is_selected = i == self.cheat.selected;
            let prefix = if is_selected { ">" } else { " " };
            let content = format!(" {} {:<width$}", prefix, label, width = box_w - 5);
            let line = format!("\u{2551}{:<width$}\u{2551}", content, width = box_w - 2);

            let (fg, row_bg) = if is_selected {
                (selected_color, sel_bg)
            } else if *is_on {
                (on_color, bg)
            } else {
                (normal_color, bg)
            };

            queue!(stdout, cursor::MoveTo(start_x as u16, y as u16),
                style::SetForegroundColor(fg), style::SetBackgroundColor(row_bg),
                style::Print(&line))?;
        }

        // Blank
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 7) as u16),
            style::SetForegroundColor(normal_color), style::SetBackgroundColor(bg),
            style::Print(&blank))?;

        // Help
        let dim_color = Color::Rgb { r: 100, g: 100, b: 120 };
        let help = format!("\u{2551}{:^width$}\u{2551}", "\u{2191}\u{2193}:Select  \u{2190}\u{2192}/Enter:Change", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 8) as u16),
            style::SetForegroundColor(dim_color), style::SetBackgroundColor(bg),
            style::Print(&help))?;

        // Close
        let close = format!("\u{2551}{:^width$}\u{2551}", "Esc:Close", width = box_w - 2);
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 9) as u16),
            style::SetForegroundColor(dim_color), style::SetBackgroundColor(bg),
            style::Print(&close))?;

        // Bottom border
        let bottom = format!("\u{255a}{}\u{255d}", "\u{2550}".repeat(box_w - 2));
        queue!(stdout, cursor::MoveTo(start_x as u16, (start_y + 10) as u16),
            style::SetForegroundColor(normal_color), style::SetBackgroundColor(bg),
            style::Print(&bottom))?;

        Ok(())
    }

    /// Handle input while in CheatMenu phase. Returns Some(stage) to launch, None to stay.
    pub fn update_cheat_menu(&mut self, input: &InputState) -> Option<usize> {
        if let Some(key) = input.last_raw_key {
            match key {
                crossterm::event::KeyCode::Up => {
                    if self.cheat.selected > 0 {
                        self.cheat.selected -= 1;
                    } else {
                        self.cheat.selected = 3;
                    }
                }
                crossterm::event::KeyCode::Down => {
                    if self.cheat.selected < 3 {
                        self.cheat.selected += 1;
                    } else {
                        self.cheat.selected = 0;
                    }
                }
                crossterm::event::KeyCode::Left => {
                    if self.cheat.selected == 0 && self.cheat.start_stage > 1 {
                        self.cheat.start_stage -= 1;
                    }
                }
                crossterm::event::KeyCode::Right => {
                    if self.cheat.selected == 0 && self.cheat.start_stage < 6 {
                        self.cheat.start_stage += 1;
                    }
                }
                crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char(' ') => {
                    match self.cheat.selected {
                        0 => {} // stage select — use left/right
                        1 => self.cheat.invincible = !self.cheat.invincible,
                        2 => self.cheat.infinite_fuel = !self.cheat.infinite_fuel,
                        3 => return Some(self.cheat.start_stage), // launch
                        _ => {}
                    }
                }
                crossterm::event::KeyCode::Esc => {
                    return Some(0); // 0 = close without launching
                }
                _ => {}
            }
        }
        None
    }

    /// Handle input while in Config phase. Returns true if should exit config.
    pub fn update_config(&mut self, input: &InputState) -> bool {
        if self.config.binding {
            // Waiting for a keypress to bind
            if let Some(key) = input.last_raw_key {
                if key == crossterm::event::KeyCode::Esc {
                    // Cancel binding
                    self.config.binding = false;
                } else {
                    self.keybinds.set(self.config.selected, key);
                    self.config.binding = false;
                }
            }
            false
        } else {
            // Navigation mode
            if let Some(key) = input.last_raw_key {
                match key {
                    crossterm::event::KeyCode::Up => {
                        if self.config.selected > 0 {
                            self.config.selected -= 1;
                        } else {
                            self.config.selected = 7;
                        }
                    }
                    crossterm::event::KeyCode::Down => {
                        if self.config.selected < 7 {
                            self.config.selected += 1;
                        } else {
                            self.config.selected = 0;
                        }
                    }
                    crossterm::event::KeyCode::Enter => {
                        self.config.binding = true;
                        self.config.blink_timer = 0.0;
                    }
                    crossterm::event::KeyCode::Esc => {
                        // Sync keybindings to save data and persist
                        for i in 0..8 {
                            self.save_data.keybindings.set(i, self.keybinds.get(i));
                        }
                        self.save_data.save();
                        return true; // exit config
                    }
                    _ => {}
                }
            }
            false
        }
    }
}
