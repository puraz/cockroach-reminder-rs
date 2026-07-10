//! Overlay lifecycle management: spawning, updating, and releasing overlay windows.
//!
//! Extracted from `main.rs` to isolate the overlay lifecycle from the app's
//! core iced daemon. Owns the overlay windows, cockroach state, and all
//! per-break animation state.

use crate::cockroach::{AnimConfig, Cockroach};
use crate::config::Settings;
use crate::overlay::{Overlay, SpriteFrame};
use crate::platform;
use crate::timer::{Phase, Timer};
use crate::Message;

use iced::window;
use iced::Task;
use rand::Rng;
use std::time::{Duration, Instant};

pub(crate) const OVERLAY_HOT_CACHE_TTL: Duration = Duration::from_secs(90);
const OVERLAY_PREWARM_BEFORE_BREAK: i64 = 5_000;

/// Manages all overlay windows and their cockroach animations.
pub struct OverlayManager {
    pub overlays: Vec<Overlay>,
    pub pending_break: bool,
    pub anim_start: Instant,
    pub rng: Option<rand::rngs::ThreadRng>,
}

impl OverlayManager {
    pub fn new() -> Self {
        Self {
            overlays: Vec::new(),
            pending_break: false,
            anim_start: Instant::now(),
            rng: Some(rand::thread_rng()),
        }
    }

    pub fn anim_config(&self, settings: &Settings) -> AnimConfig {
        AnimConfig {
            size_percent: settings.cockroach_size_percent,
            normal_fps: settings.normal_speed_fps,
            fast_min_fps: settings.fast_speed_min_fps,
            fast_max_fps: settings.fast_speed_max_fps,
            fast_probability: settings.fast_speed_probability,
            movement_percent: settings.movement_percent,
        }
    }

    pub fn any_active_overlay(&self) -> bool {
        self.overlays.iter().any(|ov| ov.active)
    }

    pub fn seed_cockroaches(
        count: u32,
        cfg: AnimConfig,
        rng: &mut impl Rng,
        width: f32,
        height: f32,
    ) -> Vec<Cockroach> {
        (0..count)
            .map(|_| Cockroach::new(rng, cfg, width, height))
            .collect()
    }

    /// Show one transparent overlay per display and seed it with `cockroach_count` roaches.
    pub fn spawn_overlays(
        &mut self,
        settings: &Settings,
        _timer: &Timer,
        frames: Option<&Vec<SpriteFrame>>,
    ) -> Task<Message> {
        if frames.is_none() {
            self.pending_break = true;
            return Task::none();
        }
        self.pending_break = false;

        // Optional system notification.
        if settings.show_notifications {
            notify("🪳 休息时间到！", "该放松一下眼睛了！看，蟑螂们出来了...");
        }

        let mut screens = platform::screen_frames();
        if screens.is_empty() {
            screens.push(platform::ScreenFrame {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            });
        }

        self.show_overlays(settings, screens)
    }

    pub fn show_overlays(
        &mut self,
        settings: &Settings,
        screens: Vec<platform::ScreenFrame>,
    ) -> Task<Message> {
        self.sync_overlays(settings, screens, true)
    }

    pub fn prewarm_overlays_with_settings(&mut self, settings: &Settings) -> Task<Message> {
        if settings.show_notifications {
            // just use settings check for consistency
        }
        let mut screens = platform::screen_frames();
        if screens.is_empty() {
            screens.push(platform::ScreenFrame {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            });
        }
        self.sync_overlays(settings, screens, false)
    }

    pub fn sync_overlays(
        &mut self,
        settings: &Settings,
        screens: Vec<platform::ScreenFrame>,
        active: bool,
    ) -> Task<Message> {
        self.anim_start = Instant::now();
        let cfg = self.anim_config(settings);
        let count = settings.cockroach_count;
        let rng = self.rng.get_or_insert_with(rand::thread_rng);
        let mut tasks = Vec::new();
        let screen_count = screens.len();

        for (i, sf) in screens.into_iter().enumerate() {
            let w = sf.width as f32;
            let h = sf.height as f32;

            if let Some(ov) = self.overlays.get_mut(i) {
                ov.width = w;
                ov.height = h;
                ov.active = active;
                ov.hidden_since = (!active).then(Instant::now);
                ov.cockroaches = if active {
                    Self::seed_cockroaches(count, cfg, rng, w, h)
                } else {
                    Vec::new()
                };
                let id = ov.id;
                tasks.push(window::move_to::<Message>(
                    id,
                    iced::Point::new(sf.x as f32, sf.y as f32),
                ));
                tasks.push(window::resize::<Message>(id, iced::Size::new(w, h)));
                let mode = if active {
                    window::Mode::Windowed
                } else {
                    window::Mode::Hidden
                };
                tasks.push(window::set_mode::<Message>(id, mode));
                tasks.push(window::set_level::<Message>(id, window::Level::AlwaysOnTop));
                tasks.push(window::run(id, move |window| {
                    if let Ok(handle) = window.window_handle() {
                        platform::configure_overlay(&handle.as_raw(), i);
                    }
                    Message::Noop
                }));
                continue;
            }

            let cockroaches = if active {
                Self::seed_cockroaches(count, cfg, rng, w, h)
            } else {
                Vec::new()
            };

            let (id, open_task) = window::open(window::Settings {
                size: iced::Size::new(w, h),
                position: window::Position::Specific(iced::Point::new(sf.x as f32, sf.y as f32)),
                transparent: true,
                decorations: false,
                resizable: false,
                level: window::Level::AlwaysOnTop,
                visible: active,
                exit_on_close_request: false,
                ..Default::default()
            });

            self.overlays.push(Overlay {
                id,
                width: w,
                height: h,
                active,
                hidden_since: (!active).then(Instant::now),
                cockroaches,
            });

            tasks.push(open_task.then(move |id| {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        platform::configure_overlay(&handle.as_raw(), i);
                    }
                    Message::Noop
                })
            }));
        }

        let excess_ids: Vec<_> = self
            .overlays
            .drain(screen_count.min(self.overlays.len())..)
            .map(|ov| ov.id)
            .collect();
        for id in excess_ids {
            tasks.push(window::close::<Message>(id));
        }

        Task::batch(tasks)
    }

    pub fn close_overlays(&mut self) -> Task<Message> {
        self.close_overlays_at(Instant::now())
    }

    pub fn close_overlays_at(&mut self, now: Instant) -> Task<Message> {
        let tasks = self.overlays.iter_mut().map(|ov| {
            ov.active = false;
            ov.cockroaches.clear();
            ov.hidden_since = Some(now);
            window::set_mode::<Message>(ov.id, window::Mode::Hidden)
        });
        Task::batch(tasks)
    }

    pub fn maintain_overlays(
        &mut self,
        timer: &Timer,
        now: Instant,
        settings: &Settings,
    ) -> Task<Message> {
        if self.any_active_overlay() {
            return Task::none();
        }

        if timer.phase == Phase::Running && timer.remaining_ms <= OVERLAY_PREWARM_BEFORE_BREAK {
            return self.prewarm_overlays_with_settings(settings);
        }

        self.release_idle_overlays(now)
    }

    pub fn release_idle_overlays(&mut self, now: Instant) -> Task<Message> {
        let mut tasks = Vec::new();
        let mut i = 0;
        while i < self.overlays.len() {
            let should_release = self.overlays[i].hidden_since.is_some_and(|hidden_since| {
                now.duration_since(hidden_since) >= OVERLAY_HOT_CACHE_TTL
            });

            if should_release {
                let ov = self.overlays.remove(i);
                tasks.push(window::close::<Message>(ov.id));
            } else {
                i += 1;
            }
        }

        Task::batch(tasks)
    }
}

fn notify(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .show();
}
