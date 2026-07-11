//! A single crawling cockroach. Physics ported 1:1 from `src/renderer/overlay/overlay.js`.

use crate::constants::{FRAME_ASPECT, TOTAL_FRAMES};
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

    pub fn el_height(&self, width: f32) -> f32 {
        self.el_width(width) * FRAME_ASPECT
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_cfg() -> AnimConfig {
        AnimConfig {
            size_percent: 35.0,
            normal_fps: 10.0,
            fast_min_fps: 10.0,
            fast_max_fps: 60.0,
            fast_probability: 0.0,
            movement_percent: 13.5,
        }
    }

    fn fast_cfg() -> AnimConfig {
        AnimConfig {
            fast_probability: 1.0,
            fast_min_fps: 30.0,
            fast_max_fps: 60.0,
            ..test_cfg()
        }
    }

    fn seeded() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn new_sets_interval_from_normal_fps() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        assert!((roach.interval_ms - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn new_sets_interval_from_fast_fps_range() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, fast_cfg(), 1920.0, 1080.0);
        let expected_min = 1000.0 / 60.0;
        let expected_max = 1000.0 / 30.0;
        assert!(roach.interval_ms >= expected_min);
        assert!(roach.interval_ms <= expected_max);
    }

    #[test]
    fn new_has_reasonable_spawn_delay() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        assert!(roach.spawn_delay_ms >= 0.0 && roach.spawn_delay_ms <= 3000.0);
    }

    #[test]
    fn new_not_drawable() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        assert!(!roach.is_drawable());
    }

    #[test]
    fn new_center_is_initial_position() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        assert_eq!(roach.center_x, roach.start_x);
        assert_eq!(roach.center_y, roach.start_y);
    }

    #[test]
    fn el_width_scales_with_viewport() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        let expected = 0.35 * 1920.0;
        assert!((roach.el_width(1920.0) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn el_width_proportional_to_width() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        let w1 = roach.el_width(1000.0);
        let w2 = roach.el_width(2000.0);
        assert!((w2 - w1 * 2.0).abs() < 0.001);
    }

    #[test]
    fn el_height_uses_aspect_ratio() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        let w = roach.el_width(1920.0);
        assert!((roach.el_height(1920.0) - w * FRAME_ASPECT).abs() < 0.001);
    }

    #[test]
    fn init_position_from_all_four_sides() {
        for seed in 0..20u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
            let on_left_or_right = roach.start_x <= -100.0 || roach.start_x >= 1920.0 + 100.0;
            let on_top_or_bottom = roach.start_y <= -100.0 || roach.start_y >= 1080.0 + 100.0;
            assert!(on_left_or_right || on_top_or_bottom);
            // Left side can produce angles as low as -45°.
            assert!(
                roach.angle_deg >= -45.0 && roach.angle_deg < 315.0,
                "angle {} out of expected range",
                roach.angle_deg
            );
        }
    }

    #[test]
    fn becomes_drawable_after_spawn_delay() {
        let mut rng = seeded();
        let mut roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        let delay = roach.spawn_delay_ms;

        roach.update(&mut rng, 0.0, 1920.0, 1080.0);
        assert!(!roach.is_drawable());

        roach.update(&mut rng, delay + 1.0, 1920.0, 1080.0);
        assert!(roach.is_drawable());
        assert!(roach.spawned);
        assert!(roach.visible);
    }

    #[test]
    fn frame_advances_after_interval() {
        let mut rng = seeded();
        let mut roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        let delay = roach.spawn_delay_ms;

        roach.update(&mut rng, delay + 1.0, 1920.0, 1080.0);
        let f0 = roach.cur_frame;

        roach.update(
            &mut rng,
            delay + roach.interval_ms + 1.0,
            1920.0,
            1080.0,
        );
        assert!(
            roach.cur_frame >= f0 + 1
                || (roach.cur_frame as usize) < crate::constants::TOTAL_FRAMES
        );
    }

    #[test]
    fn frame_wraps_around_at_total_frames() {
        let mut rng = seeded();
        let cfg = AnimConfig {
            movement_percent: 50.0,
            ..test_cfg()
        };
        let mut roach = Cockroach::new(&mut rng, cfg, 100.0, 100.0);
        let delay = roach.spawn_delay_ms;

        let total_intervals = (TOTAL_FRAMES * 3) as f32;
        roach.update(
            &mut rng,
            delay + roach.interval_ms * total_intervals,
            100.0,
            100.0,
        );

        let f = roach.cur_frame as usize;
        assert!(f < TOTAL_FRAMES, "frame {} should be < {}", f, TOTAL_FRAMES);
    }

    #[test]
    fn update_does_not_move_before_spawn() {
        let mut rng = seeded();
        let mut roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        let orig_x = roach.center_x;
        let orig_y = roach.center_y;

        roach.update(&mut rng, 0.0, 1920.0, 1080.0);
        assert_eq!(roach.center_x, orig_x);
        assert_eq!(roach.center_y, orig_y);
    }

    #[test]
    fn angle_deg_returns_angle() {
        let mut rng = seeded();
        let roach = Cockroach::new(&mut rng, test_cfg(), 1920.0, 1080.0);
        assert_eq!(roach.angle_deg(), roach.angle_deg);
    }

    #[test]
    fn respawns_when_fully_offscreen() {
        let mut rng = seeded();
        let cfg = AnimConfig {
            movement_percent: 100.0,
            ..test_cfg()
        };
        let mut roach = Cockroach::new(&mut rng, cfg, 100.0, 100.0);
        let delay = roach.spawn_delay_ms;
        let orig_start_x = roach.start_x;
        let orig_start_y = roach.start_y;

        roach.update(&mut rng, delay + 500_000.0, 100.0, 100.0);

        let respawned =
            roach.start_x != orig_start_x || roach.start_y != orig_start_y;
        assert!(respawned, "expected cockroach to respawn when off-screen");
    }

    #[test]
    fn multiple_roaches_with_same_seed_are_identical() {
        let mut rng_a = StdRng::seed_from_u64(99);
        let mut rng_b = StdRng::seed_from_u64(99);
        let a = Cockroach::new(&mut rng_a, test_cfg(), 1920.0, 1080.0);
        let b = Cockroach::new(&mut rng_b, test_cfg(), 1920.0, 1080.0);

        assert_eq!(a.start_x, b.start_x);
        assert_eq!(a.start_y, b.start_y);
        assert_eq!(a.angle_deg, b.angle_deg);
        assert_eq!(a.spawn_delay_ms, b.spawn_delay_ms);
        assert_eq!(a.interval_ms, b.interval_ms);
    }

    #[test]
    fn different_seeds_different_roaches() {
        let mut rng_a = StdRng::seed_from_u64(1);
        let mut rng_b = StdRng::seed_from_u64(2);
        let a = Cockroach::new(&mut rng_a, test_cfg(), 1920.0, 1080.0);
        let b = Cockroach::new(&mut rng_b, test_cfg(), 1920.0, 1080.0);

        let different = a.start_x != b.start_x
            || a.start_y != b.start_y
            || a.angle_deg != b.angle_deg;
        assert!(different, "different seeds should produce different roaches");
    }
}
