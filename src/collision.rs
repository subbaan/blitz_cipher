use crate::objects::{Ship, Bullet, Bomb, Enemy, TurretBullet};
use crate::terrain::Terrain;

/// Check if two axis-aligned bounding boxes overlap
fn aabb_overlap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1
}

/// Check ship vs terrain collision
pub fn ship_vs_terrain(ship: &Ship, terrain: &Terrain, viewport_h: usize) -> bool {
    if !ship.alive || ship.invincible_timer > 0.0 { return false; }

    let (left, top, right, bottom) = ship.bounding_box();

    for col_x in (left as usize)..=(right as usize) {
        if let Some(tc) = terrain.get_at(col_x) {
            // Ceiling collision
            if tc.ceiling > 0 && (top as usize) < tc.ceiling {
                return true;
            }
            // Floor collision
            let floor_row = viewport_h.saturating_sub(tc.floor);
            if tc.floor > 0 && (bottom as usize) >= floor_row {
                return true;
            }
        }
    }
    false
}

/// Check bullet vs enemies, return indices of hit enemies
pub fn bullets_vs_enemies(bullets: &mut [Bullet], enemies: &mut [Enemy]) -> Vec<(usize, u32)> {
    let mut hits = Vec::new();

    for bullet in bullets.iter_mut() {
        if !bullet.active { continue; }

        let bullet_bb = (bullet.pos.x, bullet.pos.y, bullet.pos.x + 1.0, bullet.pos.y + 1.0);

        for (i, enemy) in enemies.iter_mut().enumerate() {
            if !enemy.alive { continue; }

            if aabb_overlap(bullet_bb, enemy.bounding_box()) {
                bullet.active = false;
                let score = enemy.score_value();
                enemy.alive = false;
                hits.push((i, score));
                break;
            }
        }
    }

    hits
}

/// Check bombs vs enemies
pub fn bombs_vs_enemies(bombs: &mut [Bomb], enemies: &mut [Enemy]) -> Vec<(usize, u32)> {
    let mut hits = Vec::new();

    for bomb in bombs.iter_mut() {
        if !bomb.active { continue; }

        let bomb_bb = (bomb.pos.x, bomb.pos.y, bomb.pos.x + 1.0, bomb.pos.y + 1.0);

        for (i, enemy) in enemies.iter_mut().enumerate() {
            if !enemy.alive { continue; }

            if aabb_overlap(bomb_bb, enemy.bounding_box()) {
                bomb.active = false;
                let score = enemy.score_value();
                enemy.alive = false;
                hits.push((i, score));
                break;
            }
        }
    }

    hits
}

/// Check bombs vs terrain (floor)
pub fn bombs_vs_terrain(bombs: &mut [Bomb], terrain: &Terrain, viewport_h: usize) -> Vec<(f64, f64)> {
    let mut impacts = Vec::new();

    for bomb in bombs.iter_mut() {
        if !bomb.active { continue; }

        let col_x = bomb.pos.x as usize;
        if let Some(tc) = terrain.get_at(col_x) {
            let floor_row = viewport_h.saturating_sub(tc.floor);
            if tc.floor > 0 && bomb.pos.y as usize >= floor_row {
                impacts.push((bomb.pos.x, bomb.pos.y));
                bomb.active = false;
            }
        }
    }

    impacts
}

/// Check ship vs enemies
pub fn ship_vs_enemies(ship: &Ship, enemies: &[Enemy]) -> Option<usize> {
    if !ship.alive || ship.invincible_timer > 0.0 { return None; }

    let ship_bb = ship.bounding_box();

    for (i, enemy) in enemies.iter().enumerate() {
        if !enemy.alive { continue; }
        if aabb_overlap(ship_bb, enemy.bounding_box()) {
            return Some(i);
        }
    }
    None
}

/// Check turret bullets vs ship
pub fn turret_bullets_vs_ship(bullets: &mut [TurretBullet], ship: &Ship) -> bool {
    if !ship.alive || ship.invincible_timer > 0.0 { return false; }

    let ship_bb = ship.bounding_box();

    for bullet in bullets.iter_mut() {
        if !bullet.active { continue; }
        let bb = (bullet.pos.x, bullet.pos.y, bullet.pos.x + 1.0, bullet.pos.y + 1.0);
        if aabb_overlap(ship_bb, bb) {
            bullet.active = false;
            return true;
        }
    }
    false
}
