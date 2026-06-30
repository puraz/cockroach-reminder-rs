//! A single crawling cockroach. Physics ported 1:1 from `src/renderer/overlay/overlay.js`.

use crate::constants::TOTAL_FRAMES;
use rand::Rng;
use std::f32::consts::PI;

/// Per-cockroach animation parameters resolved from [`crate::config::Settings`].
#[derive(Debug, Clone, Copy)]
pub struct AnimConfig {
    pub size_percent: f32,
    pub normal_fps: f32,
    pub fast_min_fps: f32,
    pub fast_max_fps: f32,
    pub fast_probability: f32,
    pub movement_percent: f32,
}

pub struct Cockroach {
    cfg: AnimConfig,

    start_x: f32,
    start_y: f32,
    angle_deg: f32,
    vx: f32,
    vy: f32,

    /// Frame interval in milliseconds (`1000 / fps`).
    interval_ms: f32,
    spawn_delay_ms: f32,
    spawned: bool,
    visible: bool,
    travel_start_ms: f32,

    // --- Derived state read by the renderer ---
    pub cur_frame: usize,
    pub center_x: f32,
    pub center_y: f32,
}

impl Cockroach {
    pub fn new(rng: &mut impl Rng, cfg: AnimConfig, width: f32, height: f32) -> Self {
        // Speed: `fast_probability` chance of a fast individual, otherwise normal fps.
        let is_fast = rng.gen::<f32>() < cfg.fast_probability;
        let fps = if is_fast {
            cfg.fast_min_fps + rng.gen::<f32>() * (cfg.fast_max_fps - cfg.fast_min_fps)
        } else {
            cfg.normal_fps
        };

        let mut c = Self {
            cfg,
            start_x: 0.0,
            start_y: 0.0,
            angle_deg: 0.0,
            vx: 0.0,
            vy: 0.0,
            interval_ms: 1000.0 / fps,
            // Spawn delay (0-3 seconds stagger).
            spawn_delay_ms: rng.gen::<f32>() * 3000.0,
            spawned: false,
            visible: false,
            travel_start_ms: 0.0,
            cur_frame: 0,
            center_x: -1.0e6,
            center_y: -1.0e6,
        };
        c.init_random_position(rng, width, height);
        c
    }

    /// Displayed cockroach width in pixels (`size_percent` vw, like the original CSS).
    pub fn el_width(&self, width: f32) -> f32 {
        self.cfg.size_percent / 100.0 * width
    }

    pub fn angle_deg(&self) -> f32 {
        self.angle_deg
    }

    pub fn is_drawable(&self) -> bool {
        self.spawned && self.visible
    }

    fn init_random_position(&mut self, rng: &mut impl Rng, w: f32, h: f32) {
        let side = rng.gen_range(0..4);
        let padding = 100.0;

        let (x, y, target_angle) = match side {
            0 => {
                // Top
                (
                    rng.gen::<f32>() * w,
                    -padding,
                    90.0 + (rng.gen::<f32>() * 90.0 - 45.0),
                )
            }
            1 => {
                // Right
                (
                    w + padding,
                    rng.gen::<f32>() * h,
                    180.0 + (rng.gen::<f32>() * 90.0 - 45.0),
                )
            }
            2 => {
                // Bottom
                (
                    rng.gen::<f32>() * w,
                    h + padding,
                    270.0 + (rng.gen::<f32>() * 90.0 - 45.0),
                )
            }
            _ => {
                // Left
                (
                    -padding,
                    rng.gen::<f32>() * h,
                    0.0 + (rng.gen::<f32>() * 90.0 - 45.0),
                )
            }
        };

        self.start_x = x;
        self.start_y = y;
        self.angle_deg = target_angle;

        let rad = target_angle * PI / 180.0;
        self.vx = rad.cos();
        self.vy = rad.sin();

        self.center_x = x;
        self.center_y = y;
    }

    /// Advance the animation. `now_ms` is elapsed time since the break started.
    pub fn update(&mut self, rng: &mut impl Rng, now_ms: f32, w: f32, h: f32) {
        if !self.spawned {
            if now_ms >= self.spawn_delay_ms {
                self.spawned = true;
                self.visible = true;
                self.travel_start_ms = now_ms;
                self.update_motion(rng, now_ms, w, h);
            }
            return;
        }

        self.update_motion(rng, now_ms, w, h);
    }

    fn update_motion(&mut self, rng: &mut impl Rng, now_ms: f32, w: f32, h: f32) {
        let elapsed = (now_ms - self.travel_start_ms).max(0.0);
        let frame_progress = elapsed / self.interval_ms;
        self.cur_frame = (frame_progress.floor() as usize) % TOTAL_FRAMES;

        let el_w = self.el_width(w);
        let movement = self.cfg.movement_percent / 100.0;
        let offset = (frame_progress / TOTAL_FRAMES as f32) * (el_w * movement);

        let cur_x = offset * self.vx;
        let cur_y = offset * self.vy;
        self.center_x = self.start_x + cur_x;
        self.center_y = self.start_y + cur_y;

        // Boundary check: once fully off-screen, respawn from a fresh edge.
        let margin = el_w;
        if self.center_x < -margin
            || self.center_x > w + margin
            || self.center_y < -margin
            || self.center_y > h + margin
        {
            self.reset(rng, now_ms, w, h);
        }
    }

    fn reset(&mut self, rng: &mut impl Rng, now_ms: f32, w: f32, h: f32) {
        self.travel_start_ms = now_ms;
        self.cur_frame = 0;
        self.init_random_position(rng, w, h);
    }
}
