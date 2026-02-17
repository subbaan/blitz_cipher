#![allow(unused)]

mod audio;
mod camera;
mod collision;
mod game;
mod gamepad;
mod hud;
mod input;
mod keybinds;
mod objects;
mod parallax;
mod renderer;
mod save;
mod stages;
mod terrain;

use crossterm::{
    cursor, event, execute,
    terminal::{self, ClearType},
};
use std::io;
use std::time::{Duration, Instant};

use game::{GamePhase, GameState};
use input::InputState;

const TARGET_FPS: f64 = 30.0;
const FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000.0 / TARGET_FPS) as u64);

fn main() -> io::Result<()> {
    // Terminal setup
    terminal::enable_raw_mode()?;
    let mut stdout = io::BufWriter::new(io::stdout());
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        event::EnableMouseCapture,
    )?;

    // Enable kitty keyboard protocol for key release events
    execute!(
        stdout,
        event::PushKeyboardEnhancementFlags(
            event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | event::KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                | event::KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        ),
    ).ok(); // Silently fail if terminal doesn't support it

    let (tw, th) = terminal::size()?;
    let mut game = GameState::new(tw as usize, th as usize);
    let mut input = InputState::default();
    let mut gamepad = gamepad::GamepadHandler::new();

    let mut last_frame = Instant::now();

    // Main loop
    loop {
        let frame_start = Instant::now();
        let dt = frame_start.duration_since(last_frame).as_secs_f64();
        last_frame = frame_start;
        let dt = dt.min(0.1); // Cap delta time to prevent physics explosions

        // Handle terminal resize
        if let Ok((w, h)) = terminal::size() {
            if w as usize != game.term_width || h as usize != game.term_height {
                game.handle_resize(w as usize, h as usize);
                // Force full redraw
                execute!(stdout, terminal::Clear(ClearType::All))?;
            }
        }

        // Input
        input.poll(&game.keybinds);
        gamepad.poll(&mut input);

        // Countdown phase: block gameplay input, allow quit only
        if game.phase == GamePhase::Countdown {
            if input.quit {
                break;
            }
            game.update(dt, &input);
            game.render(&mut stdout)?;
            let elapsed = frame_start.elapsed();
            if elapsed < FRAME_DURATION {
                std::thread::sleep(FRAME_DURATION - elapsed);
            }
            continue;
        }

        // Config phase handles its own input — skip normal processing entirely
        if game.phase == GamePhase::Config {
            if game.update_config(&input) {
                game.phase = if game.config.return_to_title {
                    GamePhase::Title
                } else {
                    GamePhase::Playing
                };
                game.renderer.force_redraw();
                // Drain any remaining events so Esc doesn't leak as quit
                while event::poll(Duration::ZERO).unwrap_or(false) {
                    let _ = event::read();
                }
            }
            game.update(dt, &input);
            game.render(&mut stdout)?;
            let elapsed = frame_start.elapsed();
            if elapsed < FRAME_DURATION {
                std::thread::sleep(FRAME_DURATION - elapsed);
            }
            continue;
        }

        // Cheat menu phase handles its own input
        if game.phase == GamePhase::CheatMenu {
            if let Some(stage) = game.update_cheat_menu(&input) {
                if stage >= 1 {
                    game.start_game_at_stage(stage);
                } else {
                    game.phase = GamePhase::Title;
                }
                game.renderer.force_redraw();
                while event::poll(Duration::ZERO).unwrap_or(false) {
                    let _ = event::read();
                }
            }
            game.update(dt, &input);
            game.render(&mut stdout)?;
            let elapsed = frame_start.elapsed();
            if elapsed < FRAME_DURATION {
                std::thread::sleep(FRAME_DURATION - elapsed);
            }
            continue;
        }

        // Debug/cheat menu can be opened from Title
        if input.cheat_toggle && game.phase == GamePhase::Title {
            game.cheat.selected = 0;
            game.phase = GamePhase::CheatMenu;
            game.renderer.force_redraw();
            continue;
        }

        // Config can be opened from Title or Playing
        if input.config_toggle {
            match game.phase {
                GamePhase::Title => {
                    game.config.return_to_title = true;
                    game.config.binding = false;
                    game.phase = GamePhase::Config;
                    game.renderer.force_redraw();
                    continue;
                }
                GamePhase::Playing => {
                    game.config.return_to_title = false;
                    game.config.binding = false;
                    game.phase = GamePhase::Config;
                    game.renderer.force_redraw();
                    continue;
                }
                _ => {}
            }
        }

        if input.quit {
            break;
        }

        // Phase transitions from input
        match game.phase {
            GamePhase::Title => {
                if game.title_timer > 0.3 && (input.any_key || input.fire) {
                    game.start_game();
                }
            }
            GamePhase::GameOver => {
                if !game.game_over.entering_name && (input.any_key || input.fire) {
                    game.return_to_title();
                }
                // When entering_name, game.update() handles raw key input
            }
            _ => {}
        }

        // Update
        game.update(dt, &input);

        // Render
        game.render(&mut stdout)?;

        // Frame timing
        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    // Cleanup
    execute!(
        stdout,
        event::PopKeyboardEnhancementFlags,
    ).ok();
    execute!(
        stdout,
        event::DisableMouseCapture,
        cursor::Show,
        terminal::LeaveAlternateScreen,
    )?;
    terminal::disable_raw_mode()?;

    Ok(())
}
