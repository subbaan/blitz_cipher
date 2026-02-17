use rand::Rng;
use rodio::{OutputStream, OutputStreamHandle, buffer::SamplesBuffer, Sink};

const SAMPLE_RATE: u32 = 44100;

#[derive(Clone, Copy)]
pub enum SoundEffect {
    Shoot,
    Bomb,
    Explosion,
    PlayerDeath,
    FuelPickup,
    TurretFire,
    RocketLaunch,
    CountdownBeep,
    StartJingle,
}

pub struct AudioSystem {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
}

impl AudioSystem {
    pub fn new() -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Self {
                _stream: Some(stream),
                handle: Some(handle),
            },
            Err(_) => Self {
                _stream: None,
                handle: None,
            },
        }
    }

    pub fn play(&self, effect: SoundEffect) {
        let Some(handle) = &self.handle else { return };

        let samples = generate_effect(effect);
        let buffer = SamplesBuffer::new(1, SAMPLE_RATE, samples);

        // Play non-blocking: create a detached sink
        if let Ok(sink) = Sink::try_new(handle) {
            sink.append(buffer);
            sink.detach();
        }
    }
}

fn generate_effect(effect: SoundEffect) -> Vec<f32> {
    let mut rng = rand::thread_rng();

    match effect {
        SoundEffect::Shoot => {
            // Descending square wave sweep ~800→200Hz, ~60ms
            let freq_jitter: f64 = rng.gen_range(0.92..1.08);
            let dur_jitter: f64 = rng.gen_range(0.90..1.10);
            let duration = 0.06 * dur_jitter;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);

            let start_freq = 800.0 * freq_jitter;
            let end_freq = 200.0 * freq_jitter;

            for i in 0..num_samples {
                let t = i as f64 / SAMPLE_RATE as f64;
                let progress = i as f64 / num_samples as f64;
                let freq = start_freq + (end_freq - start_freq) * progress;
                let phase = t * freq * std::f64::consts::TAU;
                // Square wave
                let val = if phase.sin() > 0.0 { 0.3 } else { -0.3 };
                // Amplitude envelope: quick decay
                let env = 1.0 - progress * 0.5;
                samples.push((val * env) as f32);
            }
            samples
        }

        SoundEffect::Bomb => {
            // Low sine thump ~120Hz + noise tail, ~150ms
            let freq_jitter: f64 = rng.gen_range(0.95..1.05);
            let duration = 0.15;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);

            let freq = 120.0 * freq_jitter;

            for i in 0..num_samples {
                let t = i as f64 / SAMPLE_RATE as f64;
                let progress = i as f64 / num_samples as f64;
                // Low sine thump
                let sine = (t * freq * std::f64::consts::TAU).sin() * 0.4;
                // Noise tail grows as sine fades
                let noise: f64 = rng.gen_range(-1.0..1.0) * 0.15 * progress;
                // Envelope: exponential decay
                let env = (-progress * 4.0).exp();
                samples.push(((sine + noise) * env) as f32);
            }
            samples
        }

        SoundEffect::Explosion => {
            // Multi-layered explosion: sub-bass punch + mid crunch + high crack
            let dur_jitter: f64 = rng.gen_range(0.85..1.15);
            let pitch_jitter: f64 = rng.gen_range(0.90..1.10);
            let duration = 0.45 * dur_jitter;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);

            // Two-pole low-pass state for mid-frequency crunch
            let mut mid_prev1 = 0.0f64;
            let mut mid_prev2 = 0.0f64;
            // One-pole state for high-frequency crack
            let mut hi_prev = 0.0f64;

            let start_freq = 80.0 * pitch_jitter;
            let end_freq = 40.0 * pitch_jitter;

            for i in 0..num_samples {
                let t = i as f64 / SAMPLE_RATE as f64;
                let progress = i as f64 / num_samples as f64;

                // Component 1: Sub-bass punch — descending sine sweep
                let freq = start_freq + (end_freq - start_freq) * progress;
                let sub = (t * freq * std::f64::consts::TAU).sin()
                    * 0.4
                    * (-progress * 8.0).exp();

                // Component 2: Mid-frequency crunch — two-pole LP filtered noise
                let mid_cutoff = 0.35 * (1.0 - progress * 0.7); // cutoff descends
                let mid_noise: f64 = rng.gen_range(-1.0..1.0);
                mid_prev1 = mid_prev1 * (1.0 - mid_cutoff) + mid_noise * mid_cutoff;
                mid_prev2 = mid_prev2 * (1.0 - mid_cutoff) + mid_prev1 * mid_cutoff;
                let mid = mid_prev2 * 0.45 * (-progress * 4.0).exp();

                // Component 3: High-frequency crack — fast-decaying bright noise
                let hi_noise: f64 = rng.gen_range(-1.0..1.0);
                let hi_coeff = 0.7;
                hi_prev = hi_prev * (1.0 - hi_coeff) + hi_noise * hi_coeff;
                let hi = hi_prev * 0.3 * (-progress * 12.0).exp();

                // Sum and soft-clip via tanh for saturation/warmth
                let mixed = (sub + mid + hi).tanh();
                samples.push(mixed as f32);
            }
            samples
        }

        SoundEffect::PlayerDeath => {
            // Longer noise + descending tone, ~500ms
            let freq_jitter: f64 = rng.gen_range(0.95..1.05);
            let duration = 0.5;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);

            let start_freq = 400.0 * freq_jitter;
            let end_freq = 80.0 * freq_jitter;
            let mut prev = 0.0f64;

            for i in 0..num_samples {
                let t = i as f64 / SAMPLE_RATE as f64;
                let progress = i as f64 / num_samples as f64;
                // Descending tone
                let freq = start_freq + (end_freq - start_freq) * progress;
                let tone = (t * freq * std::f64::consts::TAU).sin() * 0.25;
                // Filtered noise
                let noise: f64 = rng.gen_range(-1.0..1.0);
                prev = prev * 0.7 + noise * 0.3;
                let noise_part = prev * 0.3;
                // Envelope
                let env = (-progress * 3.0).exp();
                samples.push(((tone + noise_part) * env) as f32);
            }
            samples
        }

        SoundEffect::FuelPickup => {
            // Rising sine arpeggio (3 quick notes), ~150ms
            let freq_jitter: f64 = rng.gen_range(0.95..1.05);
            let duration = 0.15;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);

            let base_freqs = [440.0, 554.0, 660.0]; // A4, C#5, E5 — major triad
            let note_len = num_samples / 3;

            for i in 0..num_samples {
                let t = i as f64 / SAMPLE_RATE as f64;
                let note_idx = (i / note_len).min(2);
                let freq = base_freqs[note_idx] * freq_jitter;
                let val = (t * freq * std::f64::consts::TAU).sin() * 0.3;
                // Per-note envelope
                let note_progress = (i % note_len) as f64 / note_len as f64;
                let env = 1.0 - note_progress * 0.3;
                samples.push((val * env) as f32);
            }
            samples
        }

        SoundEffect::TurretFire => {
            // Short noise burst, higher pitch, ~50ms
            let freq_jitter: f64 = rng.gen_range(0.92..1.08);
            let duration = 0.05;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);

            let filter_coeff = 0.6 * freq_jitter;
            let mut prev = 0.0f64;

            for i in 0..num_samples {
                let progress = i as f64 / num_samples as f64;
                let noise: f64 = rng.gen_range(-1.0..1.0);
                prev = prev * (1.0 - filter_coeff) + noise * filter_coeff;
                let env = 1.0 - progress;
                samples.push((prev * 0.35 * env) as f32);
            }
            samples
        }

        SoundEffect::RocketLaunch => {
            // Rising white noise sweep, ~200ms
            let rate_jitter: f64 = rng.gen_range(0.90..1.10);
            let duration = 0.2;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);

            let mut prev = 0.0f64;

            for i in 0..num_samples {
                let progress = i as f64 / num_samples as f64;
                let noise: f64 = rng.gen_range(-1.0..1.0);
                // Filter opens up over time (rising sweep)
                let coeff = (0.1 + progress * 0.7) * rate_jitter;
                let coeff = coeff.min(1.0);
                prev = prev * (1.0 - coeff) + noise * coeff;
                // Amplitude rises then cuts
                let env = (progress * 2.0).min(1.0) * (1.0 - (progress - 0.8).max(0.0) * 5.0).max(0.0);
                samples.push((prev * 0.4 * env) as f32);
            }
            samples
        }

        SoundEffect::CountdownBeep => {
            // Tight sine ping at ~600Hz, 120ms, sharp attack + quick exponential decay
            let duration = 0.12;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);
            let freq = 600.0;

            for i in 0..num_samples {
                let t = i as f64 / SAMPLE_RATE as f64;
                let progress = i as f64 / num_samples as f64;
                let val = (t * freq * std::f64::consts::TAU).sin();
                // Sharp attack, quick exponential decay
                let env = (-progress * 6.0).exp();
                samples.push((val * 0.35 * env) as f32);
            }
            samples
        }

        SoundEffect::StartJingle => {
            // Ascending arpeggio fanfare: C5→E5→G5→C6, ~400ms
            // Square wave for retro arcade feel
            let duration = 0.4;
            let num_samples = (SAMPLE_RATE as f64 * duration) as usize;
            let mut samples = Vec::with_capacity(num_samples);
            let notes = [523.0f64, 659.0, 784.0, 1047.0]; // C5, E5, G5, C6
            let note_len = num_samples / 4;

            for i in 0..num_samples {
                let t = i as f64 / SAMPLE_RATE as f64;
                let note_idx = (i / note_len).min(3);
                let freq = notes[note_idx];
                let note_progress = (i % note_len) as f64 / note_len as f64;

                // Square wave
                let phase = t * freq * std::f64::consts::TAU;
                let val = if phase.sin() > 0.0 { 0.3 } else { -0.3 };

                // Per-note envelope: punchy attack, fast decay
                let env = (-note_progress * 3.0).exp();

                // Final note sustained slightly longer with vibrato
                let vibrato = if note_idx == 3 {
                    1.0 + (t * 30.0 * std::f64::consts::TAU).sin() * 0.02
                } else {
                    1.0
                };

                samples.push((val * env * vibrato) as f32);
            }
            samples
        }
    }
}
