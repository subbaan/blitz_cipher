use crossterm::style::Color;
use crate::terrain::ZoneDef;

#[derive(Clone, Copy, PartialEq)]
pub enum UfoPattern {
    SineWave,
    Meteor,
    Bouncer,
    ZSweep,
    Charger,
    FormationWave,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ZoneType {
    OpenSky,
    Mountains,
    CaveEntrance,
    NarrowCave,
    FuelDepot,
    RocketBase,
    BossApproach,
}

#[derive(Clone, Copy)]
pub struct EnemyPlacement {
    pub col_offset: usize, // column within stage
    pub enemy_type: EnemyType,
    pub on_floor: bool,     // floor-mounted vs airborne
    pub ufo_pattern: UfoPattern,
    pub initial_phase: f64,          // phase offset for formation followers
    pub spawn_y_override: Option<f64>, // viewport fraction for Meteors
}

impl EnemyPlacement {
    fn ufo(col: usize, pattern: UfoPattern) -> Self {
        Self {
            col_offset: col,
            enemy_type: EnemyType::Ufo,
            on_floor: false,
            ufo_pattern: pattern,
            initial_phase: 0.0,
            spawn_y_override: None,
        }
    }

    fn rocket(col: usize) -> Self {
        Self {
            col_offset: col,
            enemy_type: EnemyType::Rocket,
            on_floor: true,
            ufo_pattern: UfoPattern::SineWave,
            initial_phase: 0.0,
            spawn_y_override: None,
        }
    }

    fn turret(col: usize) -> Self {
        Self {
            col_offset: col,
            enemy_type: EnemyType::Turret,
            on_floor: false,
            ufo_pattern: UfoPattern::SineWave,
            initial_phase: 0.0,
            spawn_y_override: None,
        }
    }

    fn fuel(col: usize) -> Self {
        Self {
            col_offset: col,
            enemy_type: EnemyType::FuelTank,
            on_floor: true,
            ufo_pattern: UfoPattern::SineWave,
            initial_phase: 0.0,
            spawn_y_override: None,
        }
    }

    fn meteor(col: usize) -> Self {
        Self {
            col_offset: col,
            enemy_type: EnemyType::Ufo,
            on_floor: false,
            ufo_pattern: UfoPattern::Meteor,
            initial_phase: 0.0,
            spawn_y_override: Some(0.05),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum EnemyType {
    Rocket,
    Ufo,
    FuelTank,
    Turret,
}

pub struct StageDefinition {
    pub zones: Vec<ZoneDef>,
    pub enemies: Vec<EnemyPlacement>,
    pub scroll_speed: f64,
}

/// Spawn a formation wave group: 4 enemies, 4 columns apart, with phase offsets.
fn formation_wave_group(base_col: usize, total_len: usize) -> Vec<EnemyPlacement> {
    let mut group = Vec::new();
    for i in 0..4 {
        let col = (base_col + i * 4).min(total_len.saturating_sub(1));
        group.push(EnemyPlacement {
            col_offset: col,
            enemy_type: EnemyType::Ufo,
            on_floor: false,
            ufo_pattern: UfoPattern::FormationWave,
            initial_phase: i as f64 * std::f64::consts::PI / 3.0,
            spawn_y_override: None,
        });
    }
    group
}

pub fn get_stage(stage_num: usize, viewport_h: usize, viewport_w: usize) -> StageDefinition {
    let difficulty = (stage_num.saturating_sub(1)) / 6;
    let stage_idx = (stage_num.saturating_sub(1)) % 6;
    let speed_mult = 1.0 + difficulty as f64 * 0.15;
    let density_mult = 1.0 + difficulty as f64 * 0.3;

    // Ensure stages are always substantially longer than the viewport
    // Base lengths are designed for ~80col terminals; scale up for wider ones
    let width_scale = (viewport_w as f64 / 80.0).max(1.0);

    match stage_idx {
        0 => stage_1_open_sky(speed_mult, density_mult, width_scale),
        1 => stage_2_mountains(speed_mult, density_mult, width_scale),
        2 => stage_3_cavern(speed_mult, density_mult, width_scale),
        3 => stage_4_narrow_caves(speed_mult, density_mult, width_scale),
        4 => stage_5_fuel_depot(speed_mult, density_mult, width_scale),
        5 => stage_6_base(speed_mult, density_mult, width_scale),
        _ => stage_1_open_sky(speed_mult, density_mult, width_scale),
    }
}

fn scaled(base: usize, width_scale: f64) -> usize {
    (base as f64 * width_scale) as usize
}

fn stage_1_open_sky(speed_mult: f64, density: f64, ws: f64) -> StageDefinition {
    let green = Color::Rgb { r: 40, g: 140, b: 40 };
    let green_hi = Color::Rgb { r: 80, g: 200, b: 80 };

    let zones = vec![
        ZoneDef {
            zone_type: ZoneType::OpenSky,
            length: scaled(350, ws),
            ceiling_start: 0, ceiling_end: 0,
            floor_start: 4, floor_end: 5,
            color: green, highlight: green_hi,
            surface_style: 0,
            undulation: 3.0, roughness: 1.0,
        },
        ZoneDef {
            zone_type: ZoneType::OpenSky,
            length: scaled(300, ws),
            ceiling_start: 0, ceiling_end: 0,
            floor_start: 5, floor_end: 8,
            color: green, highlight: green_hi,
            surface_style: 0,
            undulation: 3.0, roughness: 1.0,
        },
        ZoneDef {
            zone_type: ZoneType::Mountains,
            length: scaled(250, ws),
            ceiling_start: 0, ceiling_end: 0,
            floor_start: 8, floor_end: 4,
            color: green, highlight: green_hi,
            surface_style: 1,
            undulation: 3.0, roughness: 1.0,
        },
    ];

    let enemy_count = scaled(65, density * ws);
    let mut enemies = Vec::new();
    let total_len: usize = zones.iter().map(|z| z.length).sum();
    for i in 0..enemy_count {
        let col = 20 + (i * (total_len - 40) / enemy_count.max(1));
        let col = col.min(total_len - 1);
        let pct = col as f64 / total_len as f64;

        if i % 3 != 0 {
            // UFO — section-based pattern (2/3 flying)
            let pattern = if pct < 0.30 {
                UfoPattern::SineWave
            } else if pct < 0.50 {
                // Mix Meteors with SineWave so this section doesn't feel empty
                if i % 4 < 2 { UfoPattern::Meteor } else { UfoPattern::SineWave }
            } else if pct < 0.75 {
                UfoPattern::SineWave
            } else {
                if i % 6 < 3 { UfoPattern::Meteor } else { UfoPattern::SineWave }
            };
            if pattern == UfoPattern::Meteor {
                enemies.push(EnemyPlacement::meteor(col));
            } else {
                enemies.push(EnemyPlacement::ufo(col, pattern));
            }
        } else {
            enemies.push(EnemyPlacement::rocket(col));
        }
    }
    let fuel_count = (total_len / 150).max(4);
    for i in 0..fuel_count {
        let col = (i + 1) * total_len / (fuel_count + 1);
        enemies.push(EnemyPlacement::fuel(col.min(total_len - 1)));
    }

    StageDefinition { zones, enemies, scroll_speed: 15.0 * speed_mult }
}

fn stage_2_mountains(speed_mult: f64, density: f64, ws: f64) -> StageDefinition {
    let brown = Color::Rgb { r: 140, g: 90, b: 40 };
    let brown_hi = Color::Rgb { r: 200, g: 140, b: 60 };

    let zones = vec![
        ZoneDef {
            zone_type: ZoneType::Mountains,
            length: scaled(300, ws),
            ceiling_start: 0, ceiling_end: 0,
            floor_start: 5, floor_end: 10,
            color: brown, highlight: brown_hi,
            surface_style: 1,
            undulation: 6.0, roughness: 2.0,
        },
        ZoneDef {
            zone_type: ZoneType::Mountains,
            length: scaled(300, ws),
            ceiling_start: 0, ceiling_end: 0,
            floor_start: 10, floor_end: 6,
            color: brown, highlight: brown_hi,
            surface_style: 2,
            undulation: 6.0, roughness: 2.0,
        },
        ZoneDef {
            zone_type: ZoneType::OpenSky,
            length: scaled(250, ws),
            ceiling_start: 0, ceiling_end: 0,
            floor_start: 6, floor_end: 4,
            color: brown, highlight: brown_hi,
            surface_style: 1,
            undulation: 6.0, roughness: 2.0,
        },
    ];

    let enemy_count = scaled(60, density * ws);
    let mut enemies = Vec::new();
    let total_len: usize = zones.iter().map(|z| z.length).sum();
    for i in 0..enemy_count {
        let col = 30 + (i * (total_len - 60) / enemy_count.max(1));
        let col = col.min(total_len - 1);
        let pct = col as f64 / total_len as f64;

        match i % 3 {
            0 => {
                // First UFO slot — section-based pattern (2/3 flying)
                let pattern = if pct < 0.30 {
                    if i % 8 < 4 { UfoPattern::SineWave } else { UfoPattern::Bouncer }
                } else if pct < 0.50 {
                    UfoPattern::Bouncer
                } else if pct < 0.75 {
                    UfoPattern::SineWave
                } else {
                    UfoPattern::Charger
                };
                enemies.push(EnemyPlacement::ufo(col, pattern));
            }
            1 => {
                // Second UFO slot — complementary pattern
                let pattern = if pct < 0.30 {
                    if i % 8 < 4 { UfoPattern::Bouncer } else { UfoPattern::SineWave }
                } else if pct < 0.50 {
                    UfoPattern::SineWave
                } else if pct < 0.75 {
                    UfoPattern::Bouncer
                } else {
                    UfoPattern::Bouncer
                };
                enemies.push(EnemyPlacement::ufo(col, pattern));
            }
            _ => enemies.push(EnemyPlacement::rocket(col)),
        }
    }
    let fuel_count = (total_len / 150).max(4);
    for i in 0..fuel_count {
        let col = (i + 1) * total_len / (fuel_count + 1);
        enemies.push(EnemyPlacement::fuel(col.min(total_len - 1)));
    }

    StageDefinition { zones, enemies, scroll_speed: 16.0 * speed_mult }
}

fn stage_3_cavern(speed_mult: f64, density: f64, ws: f64) -> StageDefinition {
    let grey = Color::Rgb { r: 100, g: 100, b: 110 };
    let grey_hi = Color::Rgb { r: 160, g: 160, b: 180 };

    let zones = vec![
        ZoneDef {
            zone_type: ZoneType::OpenSky,
            length: scaled(200, ws),
            ceiling_start: 0, ceiling_end: 0,
            floor_start: 4, floor_end: 5,
            color: grey, highlight: grey_hi,
            surface_style: 3,
            undulation: 2.5, roughness: 1.5,
        },
        ZoneDef {
            zone_type: ZoneType::CaveEntrance,
            length: scaled(250, ws),
            ceiling_start: 0, ceiling_end: 4,
            floor_start: 5, floor_end: 6,
            color: grey, highlight: grey_hi,
            surface_style: 3,
            undulation: 2.5, roughness: 1.5,
        },
        ZoneDef {
            zone_type: ZoneType::NarrowCave,
            length: scaled(350, ws),
            ceiling_start: 4, ceiling_end: 5,
            floor_start: 6, floor_end: 5,
            color: grey, highlight: grey_hi,
            surface_style: 4,
            undulation: 2.5, roughness: 1.5,
        },
    ];

    let enemy_count = scaled(55, density * ws);
    let mut enemies = Vec::new();
    let total_len: usize = zones.iter().map(|z| z.length).sum();
    for i in 0..enemy_count {
        let col = 60 + (i * (total_len - 80) / enemy_count.max(1));
        let col = col.min(total_len - 1);
        let pct = col as f64 / total_len as f64;

        match i % 3 {
            0 => {
                // First UFO slot — section-based pattern (2/3 flying)
                let pattern = if pct < 0.30 {
                    UfoPattern::SineWave
                } else if pct < 0.50 {
                    UfoPattern::ZSweep
                } else if pct < 0.75 {
                    UfoPattern::FormationWave // individual spawns; groups added below
                } else {
                    UfoPattern::ZSweep
                };
                if pattern == UfoPattern::FormationWave {
                    // Skip individual; formation groups are inserted below
                } else {
                    enemies.push(EnemyPlacement::ufo(col, pattern));
                }
            }
            1 => {
                // Second UFO slot — SineWave filler
                enemies.push(EnemyPlacement::ufo(col, UfoPattern::SineWave));
            }
            _ => enemies.push(EnemyPlacement::turret(col)),
        }
    }

    // Insert FormationWave groups in the 50-75% section
    let fw_start = (total_len as f64 * 0.50) as usize;
    let fw_end = (total_len as f64 * 0.75) as usize;
    let fw_span = fw_end - fw_start;
    let group_count = (fw_span / 60).max(2);
    for g in 0..group_count {
        let base_col = fw_start + g * fw_span / group_count;
        enemies.extend(formation_wave_group(base_col, total_len));
    }

    let fuel_count = (total_len / 150).max(4);
    for i in 0..fuel_count {
        let col = (i + 1) * total_len / (fuel_count + 1);
        enemies.push(EnemyPlacement::fuel(col.min(total_len - 1)));
    }

    StageDefinition { zones, enemies, scroll_speed: 14.0 * speed_mult }
}

fn stage_4_narrow_caves(speed_mult: f64, density: f64, ws: f64) -> StageDefinition {
    let dark_blue = Color::Rgb { r: 40, g: 50, b: 100 };
    let blue_hi = Color::Rgb { r: 80, g: 100, b: 180 };

    let zones = vec![
        ZoneDef {
            zone_type: ZoneType::NarrowCave,
            length: scaled(300, ws),
            ceiling_start: 4, ceiling_end: 6,
            floor_start: 5, floor_end: 6,
            color: dark_blue, highlight: blue_hi,
            surface_style: 4,
            undulation: 3.5, roughness: 2.5,
        },
        ZoneDef {
            zone_type: ZoneType::NarrowCave,
            length: scaled(300, ws),
            ceiling_start: 6, ceiling_end: 4,
            floor_start: 6, floor_end: 7,
            color: dark_blue, highlight: blue_hi,
            surface_style: 5,
            undulation: 3.5, roughness: 2.5,
        },
        ZoneDef {
            zone_type: ZoneType::NarrowCave,
            length: scaled(250, ws),
            ceiling_start: 4, ceiling_end: 3,
            floor_start: 7, floor_end: 4,
            color: dark_blue, highlight: blue_hi,
            surface_style: 4,
            undulation: 3.5, roughness: 2.5,
        },
    ];

    let enemy_count = scaled(65, density * ws);
    let mut enemies = Vec::new();
    let total_len: usize = zones.iter().map(|z| z.length).sum();
    for i in 0..enemy_count {
        let col = 20 + (i * (total_len - 40) / enemy_count.max(1));
        let col = col.min(total_len - 1);
        let pct = col as f64 / total_len as f64;

        match i % 3 {
            0 => {
                // First UFO slot — section-based pattern (2/3 flying)
                let pattern = if pct < 0.30 {
                    if i % 8 < 4 { UfoPattern::Charger } else { UfoPattern::Bouncer }
                } else if pct < 0.50 {
                    UfoPattern::FormationWave
                } else if pct < 0.75 {
                    UfoPattern::ZSweep
                } else {
                    UfoPattern::Charger
                };
                if pattern == UfoPattern::FormationWave {
                    // Groups inserted below
                } else {
                    enemies.push(EnemyPlacement::ufo(col, pattern));
                }
            }
            1 => {
                // Second UFO slot — Bouncer filler
                enemies.push(EnemyPlacement::ufo(col, UfoPattern::Bouncer));
            }
            _ => enemies.push(EnemyPlacement::turret(col)),
        }
    }

    // FormationWave groups in 30-50% section
    let fw_start = (total_len as f64 * 0.30) as usize;
    let fw_end = (total_len as f64 * 0.50) as usize;
    let fw_span = fw_end - fw_start;
    let group_count = (fw_span / 60).max(2);
    for g in 0..group_count {
        let base_col = fw_start + g * fw_span / group_count;
        enemies.extend(formation_wave_group(base_col, total_len));
    }

    let fuel_count = (total_len / 150).max(4);
    for i in 0..fuel_count {
        let col = (i + 1) * total_len / (fuel_count + 1);
        enemies.push(EnemyPlacement::fuel(col.min(total_len - 1)));
    }

    StageDefinition { zones, enemies, scroll_speed: 14.0 * speed_mult }
}

fn stage_5_fuel_depot(speed_mult: f64, density: f64, ws: f64) -> StageDefinition {
    let red = Color::Rgb { r: 160, g: 60, b: 30 };
    let red_hi = Color::Rgb { r: 220, g: 100, b: 50 };

    let zones = vec![
        ZoneDef {
            zone_type: ZoneType::CaveEntrance,
            length: scaled(200, ws),
            ceiling_start: 3, ceiling_end: 2,
            floor_start: 5, floor_end: 4,
            color: red, highlight: red_hi,
            surface_style: 2,
            undulation: 1.5, roughness: 0.5,
        },
        ZoneDef {
            zone_type: ZoneType::FuelDepot,
            length: scaled(350, ws),
            ceiling_start: 2, ceiling_end: 2,
            floor_start: 4, floor_end: 4,
            color: red, highlight: red_hi,
            surface_style: 0,
            undulation: 1.5, roughness: 0.5,
        },
        ZoneDef {
            zone_type: ZoneType::OpenSky,
            length: scaled(200, ws),
            ceiling_start: 2, ceiling_end: 0,
            floor_start: 4, floor_end: 4,
            color: red, highlight: red_hi,
            surface_style: 2,
            undulation: 1.5, roughness: 0.5,
        },
    ];

    let enemy_count = scaled(50, density * ws);
    let mut enemies = Vec::new();
    let total_len: usize = zones.iter().map(|z| z.length).sum();
    for i in 0..enemy_count {
        let col = 30 + (i * (total_len - 60) / enemy_count.max(1));
        let col = col.min(total_len - 1);
        let pct = col as f64 / total_len as f64;

        match i % 3 {
            0 => {
                // First UFO slot — section-based pattern (2/3 flying)
                let pattern = if pct < 0.30 {
                    UfoPattern::FormationWave
                } else if pct < 0.50 {
                    if i % 6 < 3 { UfoPattern::ZSweep } else { UfoPattern::Meteor }
                } else if pct < 0.75 {
                    UfoPattern::Charger
                } else {
                    UfoPattern::FormationWave
                };
                if pattern == UfoPattern::FormationWave {
                    // Groups inserted below
                } else if pattern == UfoPattern::Meteor {
                    enemies.push(EnemyPlacement::meteor(col));
                } else {
                    enemies.push(EnemyPlacement::ufo(col, pattern));
                }
            }
            1 => {
                // Second UFO slot — ZSweep filler (skip for Formation sections)
                let pattern = if pct < 0.30 {
                    None // Formation section, skip
                } else if pct < 0.50 {
                    Some(UfoPattern::ZSweep)
                } else if pct < 0.75 {
                    Some(UfoPattern::ZSweep)
                } else {
                    None // Formation section, skip
                };
                if let Some(p) = pattern {
                    enemies.push(EnemyPlacement::ufo(col, p));
                }
            }
            _ => enemies.push(EnemyPlacement::rocket(col)),
        }
    }

    // FormationWave groups in 0-30% and 75-100% sections
    let fw_sections = [
        ((total_len as f64 * 0.05) as usize, (total_len as f64 * 0.28) as usize),
        ((total_len as f64 * 0.77) as usize, (total_len as f64 * 0.95) as usize),
    ];
    for (fw_start, fw_end) in &fw_sections {
        let fw_span = fw_end - fw_start;
        let group_count = (fw_span / 60).max(2);
        for g in 0..group_count {
            let base_col = fw_start + g * fw_span / group_count;
            enemies.extend(formation_wave_group(base_col, total_len));
        }
    }

    // Fuel depot stage — generous fuel, one every ~100 columns
    let fuel_count = (total_len / 100).max(6);
    for i in 0..fuel_count {
        let col = (i + 1) * total_len / (fuel_count + 1);
        enemies.push(EnemyPlacement::fuel(col.min(total_len - 1)));
    }

    StageDefinition { zones, enemies, scroll_speed: 15.0 * speed_mult }
}

fn stage_6_base(speed_mult: f64, density: f64, ws: f64) -> StageDefinition {
    let purple = Color::Rgb { r: 120, g: 40, b: 140 };
    let purple_hi = Color::Rgb { r: 180, g: 80, b: 200 };

    let zones = vec![
        ZoneDef {
            zone_type: ZoneType::NarrowCave,
            length: scaled(250, ws),
            ceiling_start: 5, ceiling_end: 6,
            floor_start: 5, floor_end: 6,
            color: purple, highlight: purple_hi,
            surface_style: 5,
            undulation: 4.0, roughness: 2.0,
        },
        ZoneDef {
            zone_type: ZoneType::RocketBase,
            length: scaled(300, ws),
            ceiling_start: 6, ceiling_end: 7,
            floor_start: 6, floor_end: 7,
            color: purple, highlight: purple_hi,
            surface_style: 3,
            undulation: 4.0, roughness: 2.0,
        },
        ZoneDef {
            zone_type: ZoneType::BossApproach,
            length: scaled(200, ws),
            ceiling_start: 7, ceiling_end: 5,
            floor_start: 7, floor_end: 4,
            color: purple, highlight: purple_hi,
            surface_style: 5,
            undulation: 4.0, roughness: 2.0,
        },
    ];

    let all_patterns = [
        UfoPattern::SineWave, UfoPattern::Meteor, UfoPattern::Bouncer,
        UfoPattern::ZSweep, UfoPattern::Charger, UfoPattern::FormationWave,
    ];

    let enemy_count = scaled(70, density * ws);
    let mut enemies = Vec::new();
    let total_len: usize = zones.iter().map(|z| z.length).sum();
    for i in 0..enemy_count {
        let col = 20 + (i * (total_len - 40) / enemy_count.max(1));
        let col = col.min(total_len - 1);

        match i % 3 {
            0 => {
                // First UFO slot — all patterns cycling (2/3 flying)
                let pattern = all_patterns[i % 6];
                if pattern == UfoPattern::Meteor {
                    enemies.push(EnemyPlacement::meteor(col));
                } else {
                    enemies.push(EnemyPlacement::ufo(col, pattern));
                }
            }
            1 => {
                // Second UFO slot — SineWave filler
                enemies.push(EnemyPlacement::ufo(col, UfoPattern::SineWave));
            }
            _ => enemies.push(EnemyPlacement::turret(col)),
        }
    }
    let fuel_count = (total_len / 150).max(4);
    for i in 0..fuel_count {
        let col = (i + 1) * total_len / (fuel_count + 1);
        enemies.push(EnemyPlacement::fuel(col.min(total_len - 1)));
    }

    StageDefinition { zones, enemies, scroll_speed: 18.0 * speed_mult }
}
