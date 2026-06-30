// 🪳 蟑螂提醒 (Cockroach Reminder) — Rust + iced rewrite.
//
// A menu-bar break reminder. When a work interval elapses, a swarm of cockroaches
// crawls across every display for the configured duration, then the timer restarts.
// Faithful port of the original Electron app (see ./cockroach-reminder).

#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod cockroach;
mod config;
mod constants;
mod overlay;
mod platform;
mod settings_ui;
mod timer;
mod tray;

use cockroach::{AnimConfig, Cockroach};
use config::Settings;
use overlay::{Overlay, OverlayCanvas, SpriteFrame};
use timer::{Phase, Timer, Transition};
use tray::{Tray, TrayCommand};

use ::image::imageops::FilterType;
use iced::widget::image as iced_image;
use iced::widget::{canvas, Space};
use iced::{window, Color, Element, Length, Subscription, Task, Theme};
use rand::Rng;
use std::time::{Duration, Instant};

const MAX_SPRITE_FRAME_WIDTH: u32 = 640;

fn main() -> iced::Result {
    iced::daemon(App::new, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .style(App::style)
        .default_font(iced::Font::with_name("PingFang SC"))
        .run()
}

#[derive(Clone)]
enum Message {
    Tick,
    Anim(Instant),
    PollTray,
    FramesLoaded(Vec<SpriteFrame>),

    SettingsOpened(window::Id),
    WindowClosed(window::Id),

    // Settings form edits
    IntervalChanged(u32),
    DurationChanged(u32),
    CountChanged(u32),
    SizeChanged(f32),
    SpeedChanged(f32),
    FastProbChanged(u32),
    AutoStartToggled(bool),
    LaunchAtLoginToggled(bool),
    ShowNotificationsToggled(bool),
    SaveSettings,
    TestBreak,
    TogglePause,

    Noop,
}

struct App {
    settings: Settings,
    /// Working copy edited by the settings form; persisted only on "保存设置".
    edit: Settings,
    timer: Timer,
    tray: Option<Tray>,
    frames: Option<Vec<SpriteFrame>>,
    pending_break: bool,

    settings_window: Option<window::Id>,
    overlays: Vec<Overlay>,
    anim_start: Instant,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let settings = Settings::load();
        let mut timer = Timer::new(settings.interval_minutes, settings.duration_seconds);
        // Menu-bar-only app (no dock icon), matching `app.dock.hide()`.
        platform::hide_dock();

        // Auto-start the timer if configured.
        if settings.auto_start {
            timer.start();
        }

        let mut app = App {
            edit: settings.clone(),
            settings,
            timer,
            tray: None,
            frames: None,
            pending_break: false,
            settings_window: None,
            overlays: Vec::new(),
            anim_start: Instant::now(),
        };

        let (status, toggle_label, toggle_enabled, tooltip) = app.tray_labels();
        app.tray = Tray::new(&status, &toggle_label, toggle_enabled, &tooltip);

        (app, load_frames_task())
    }

    fn title(&self, window: window::Id) -> String {
        if Some(window) == self.settings_window {
            "🪳 蟑螂提醒设置".to_string()
        } else {
            "Cockroach Overlay".to_string()
        }
    }

    fn theme(&self, _window: window::Id) -> Theme {
        Theme::Dark
    }

    fn style(&self, _theme: &Theme) -> iced::theme::Style {
        iced::theme::Style {
            background_color: Color::TRANSPARENT,
            text_color: Color::WHITE,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick),
            iced::time::every(Duration::from_millis(150)).map(|_| Message::PollTray),
            window::close_events().map(Message::WindowClosed),
        ];
        if self.any_active_overlay() {
            subs.push(iced::time::every(Duration::from_millis(16)).map(Message::Anim));
        }
        Subscription::batch(subs)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let task = match self.timer.tick() {
                    Some(Transition::EnteredBreak) => self.spawn_overlays(),
                    Some(Transition::EnteredRunning) => self.close_overlays(),
                    None => Task::none(),
                };
                self.refresh_tray();
                task
            }

            Message::Anim(now) => {
                if self.timer.phase == Phase::Break {
                    let elapsed_ms = now.duration_since(self.anim_start).as_secs_f32() * 1000.0;
                    let mut rng = rand::thread_rng();
                    for ov in &mut self.overlays {
                        let (w, h) = (ov.width, ov.height);
                        for roach in &mut ov.cockroaches {
                            roach.update(&mut rng, elapsed_ms, w, h);
                        }
                    }
                }
                Task::none()
            }

            Message::PollTray => match tray::poll_command() {
                Some(TrayCommand::ToggleTimer) => self.update(Message::TogglePause),
                Some(TrayCommand::TriggerBreak) => self.update(Message::TestBreak),
                Some(TrayCommand::OpenSettings) => self.open_settings(),
                Some(TrayCommand::Quit) => {
                    self.timer.stop();
                    iced::exit()
                }
                None => Task::none(),
            },

            Message::FramesLoaded(frames) => {
                self.frames = Some(frames);
                if self.pending_break && self.timer.phase == Phase::Break {
                    self.pending_break = false;
                    let task = self.spawn_overlays();
                    self.refresh_tray();
                    task
                } else {
                    Task::none()
                }
            }

            Message::SettingsOpened(id) => {
                self.settings_window = Some(id);
                Task::none()
            }

            Message::WindowClosed(id) => {
                if Some(id) == self.settings_window {
                    self.settings_window = None;
                }
                self.overlays.retain(|ov| ov.id != id);
                Task::none()
            }

            Message::IntervalChanged(v) => {
                self.edit.interval_minutes = v;
                Task::none()
            }
            Message::DurationChanged(v) => {
                self.edit.duration_seconds = v;
                Task::none()
            }
            Message::CountChanged(v) => {
                self.edit.cockroach_count = v;
                Task::none()
            }
            Message::SizeChanged(v) => {
                self.edit.cockroach_size_percent = v;
                Task::none()
            }
            Message::SpeedChanged(v) => {
                self.edit.movement_percent = v;
                Task::none()
            }
            Message::FastProbChanged(v) => {
                self.edit.fast_speed_probability = v as f32 / 100.0;
                Task::none()
            }
            Message::AutoStartToggled(v) => {
                self.edit.auto_start = v;
                Task::none()
            }
            Message::LaunchAtLoginToggled(v) => {
                self.edit.launch_at_login = v;
                Task::none()
            }
            Message::ShowNotificationsToggled(v) => {
                self.edit.show_notifications = v;
                Task::none()
            }

            Message::SaveSettings => {
                self.edit.clamp();
                self.settings = self.edit.clone();
                self.settings.save();
                self.timer.update_interval(self.settings.interval_minutes);
                self.timer.update_duration(self.settings.duration_seconds);
                self.refresh_tray();
                Task::none()
            }

            Message::TestBreak => {
                self.timer.trigger_break();
                let task = self.spawn_overlays();
                self.refresh_tray();
                task
            }

            Message::TogglePause => {
                match self.timer.phase {
                    Phase::Running => self.timer.pause(),
                    Phase::Paused => self.timer.resume(),
                    Phase::Idle => self.timer.start(),
                    Phase::Break => {}
                }
                self.refresh_tray();
                Task::none()
            }

            Message::Noop => Task::none(),
        }
    }

    fn view(&self, window: window::Id) -> Element<'_, Message> {
        if Some(window) == self.settings_window {
            self.view_settings()
        } else if let Some((ov, frames)) = self
            .overlays
            .iter()
            .find(|o| o.id == window && o.active)
            .zip(self.frames.as_deref())
        {
            canvas(OverlayCanvas {
                overlay: ov,
                frames,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            Space::new().width(Length::Fill).height(Length::Fill).into()
        }
    }

    // --- Overlay lifecycle ---

    fn anim_config(&self) -> AnimConfig {
        AnimConfig {
            size_percent: self.settings.cockroach_size_percent,
            normal_fps: self.settings.normal_speed_fps,
            fast_min_fps: self.settings.fast_speed_min_fps,
            fast_max_fps: self.settings.fast_speed_max_fps,
            fast_probability: self.settings.fast_speed_probability,
            movement_percent: self.settings.movement_percent,
        }
    }

    fn any_active_overlay(&self) -> bool {
        self.overlays.iter().any(|ov| ov.active)
    }

    fn seed_cockroaches(
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
    fn spawn_overlays(&mut self) -> Task<Message> {
        if self.frames.is_none() {
            self.pending_break = true;
            return Task::none();
        }
        self.pending_break = false;

        // Optional system notification (silent), matching the original.
        if self.settings.show_notifications {
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

        self.show_overlays(screens)
    }

    fn show_overlays(&mut self, screens: Vec<platform::ScreenFrame>) -> Task<Message> {
        self.anim_start = Instant::now();
        let cfg = self.anim_config();
        let count = self.settings.cockroach_count;
        let mut rng = rand::thread_rng();
        let mut tasks = Vec::new();
        let screen_count = screens.len();

        for (i, sf) in screens.into_iter().enumerate() {
            let w = sf.width as f32;
            let h = sf.height as f32;

            if let Some(ov) = self.overlays.get_mut(i) {
                ov.width = w;
                ov.height = h;
                ov.active = true;
                ov.cockroaches = Self::seed_cockroaches(count, cfg, &mut rng, w, h);
                let id = ov.id;
                tasks.push(window::move_to::<Message>(
                    id,
                    iced::Point::new(sf.x as f32, sf.y as f32),
                ));
                tasks.push(window::resize::<Message>(id, iced::Size::new(w, h)));
                tasks.push(window::set_mode::<Message>(id, window::Mode::Windowed));
                tasks.push(window::set_level::<Message>(id, window::Level::AlwaysOnTop));
                tasks.push(window::run(id, move |window| {
                    if let Ok(handle) = window.window_handle() {
                        platform::configure_overlay(&handle.as_raw(), i);
                    }
                    Message::Noop
                }));
                continue;
            }

            let cockroaches = Self::seed_cockroaches(count, cfg, &mut rng, w, h);

            let (id, open_task) = window::open(window::Settings {
                size: iced::Size::new(w, h),
                position: window::Position::Specific(iced::Point::new(sf.x as f32, sf.y as f32)),
                transparent: true,
                decorations: false,
                resizable: false,
                level: window::Level::AlwaysOnTop,
                visible: true,
                exit_on_close_request: false,
                ..Default::default()
            });

            self.overlays.push(Overlay {
                id,
                width: w,
                height: h,
                active: true,
                cockroaches,
            });

            // Once opened, configure the overlay via the native window handle
            // (platform-specific: macOS uses objc2, Windows uses Win32, Linux uses X11).
            tasks.push(open_task.then(move |id| {
                window::run(id, move |w| {
                    if let Ok(handle) = w.window_handle() {
                        platform::configure_overlay(&handle.as_raw(), i);
                    }
                    Message::Noop
                })
            }));
        }

        for ov in self.overlays.iter_mut().skip(screen_count) {
            ov.active = false;
            ov.cockroaches.clear();
            tasks.push(window::set_mode::<Message>(ov.id, window::Mode::Hidden));
        }

        Task::batch(tasks)
    }

    fn close_overlays(&mut self) -> Task<Message> {
        let tasks = self.overlays.iter_mut().map(|ov| {
            ov.active = false;
            ov.cockroaches.clear();
            window::set_mode::<Message>(ov.id, window::Mode::Hidden)
        });
        Task::batch(tasks)
    }

    // --- Settings window ---

    fn open_settings(&mut self) -> Task<Message> {
        if let Some(id) = self.settings_window {
            return window::gain_focus::<Message>(id);
        }
        self.edit = self.settings.clone();
        let (id, task) = window::open(window::Settings {
            size: iced::Size::new(580.0, 720.0),
            min_size: Some(iced::Size::new(500.0, 600.0)),
            resizable: true,
            transparent: false,
            decorations: true,
            exit_on_close_request: true,
            icon: app_icon(),
            ..Default::default()
        });
        self.settings_window = Some(id);
        Task::batch([task.map(Message::SettingsOpened), window::gain_focus(id)])
    }

    // --- Tray helpers ---

    fn tray_labels(&self) -> (String, String, bool, String) {
        let f = self.timer.formatted();
        let (status, tooltip) = match self.timer.phase {
            Phase::Running => (
                format!("⏱ 下次休息还有 {f}"),
                format!("🪳 下次休息还有 {f}"),
            ),
            Phase::Break => (
                format!("🪳 休息中！还剩 {f}"),
                format!("🪳 休息时间！还剩 {f}"),
            ),
            Phase::Paused => (
                format!("⏸ 已暂停 — 剩余 {f}"),
                format!("🪳 已暂停 — 剩余 {f}"),
            ),
            Phase::Idle => (
                "⏹ 计时器已停止".to_string(),
                "🪳 蟑螂提醒 (已停止)".to_string(),
            ),
        };

        let is_running = self.timer.phase == Phase::Running;
        let is_paused = self.timer.phase == Phase::Paused;
        let toggle_label = if is_running {
            "⏸  暂停计时"
        } else {
            "▶  恢复计时"
        };
        let toggle_enabled = is_running || is_paused;

        (status, toggle_label.to_string(), toggle_enabled, tooltip)
    }

    fn refresh_tray(&self) {
        if let Some(tray) = &self.tray {
            let (status, toggle_label, toggle_enabled, tooltip) = self.tray_labels();
            tray.refresh(&status, &toggle_label, toggle_enabled, &tooltip);
        }
    }

    // --- Settings UI ---

    fn view_settings(&self) -> Element<'_, Message> {
        settings_ui::view(&self.edit, self.timer.phase, &self.timer.formatted())
    }
}

fn notify(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .show();
}

fn app_icon() -> Option<window::Icon> {
    let img = ::image::load_from_memory(constants::APP_ICON_BYTES)
        .ok()?
        .into_rgba8();
    let (width, height) = img.dimensions();
    window::icon::from_rgba(img.into_raw(), width, height).ok()
}

fn load_frames_task() -> Task<Message> {
    let (sender, receiver) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(load_sprite_frames(&constants::FRAME_BYTES));
    });

    Task::perform(
        async move { receiver.await.unwrap_or_default() },
        Message::FramesLoaded,
    )
}

fn load_sprite_frames(bytes: &[&[u8]]) -> Vec<SpriteFrame> {
    let images: Vec<::image::RgbaImage> = bytes
        .iter()
        .map(|bytes| ::image::load_from_memory(bytes).unwrap().into_rgba8())
        .collect();

    let (global_min_x, global_min_y, global_max_x, global_max_y) =
        images.iter().fold((u32::MAX, u32::MAX, 0, 0), |acc, img| {
            let (min_x, min_y, max_x, max_y) = alpha_bounds(img);
            (
                acc.0.min(min_x),
                acc.1.min(min_y),
                acc.2.max(max_x),
                acc.3.max(max_y),
            )
        });

    let crop_w = global_max_x - global_min_x + 1;
    let crop_h = global_max_y - global_min_y + 1;

    images
        .iter()
        .map(|img| {
            let (body_x, body_y) = alpha_centroid(img);
            let mut crop =
                ::image::imageops::crop_imm(img, global_min_x, global_min_y, crop_w, crop_h)
                    .to_image();
            let scale = if crop_w > MAX_SPRITE_FRAME_WIDTH {
                MAX_SPRITE_FRAME_WIDTH as f32 / crop_w as f32
            } else {
                1.0
            };

            if scale < 1.0 {
                let resized_w = MAX_SPRITE_FRAME_WIDTH;
                let resized_h = (crop_h as f32 * scale).round().max(1.0) as u32;
                crop = ::image::imageops::resize(&crop, resized_w, resized_h, FilterType::Lanczos3);
            }

            let (frame_w, frame_h) = crop.dimensions();

            SpriteFrame {
                handle: iced_image::Handle::from_rgba(frame_w, frame_h, crop.into_raw()),
                width: frame_w as f32,
                height: frame_h as f32,
                body_anchor_x: (body_x - global_min_x as f32) * scale,
                body_anchor_y: (body_y - global_min_y as f32) * scale,
            }
        })
        .collect()
}

fn alpha_bounds(img: &::image::RgbaImage) -> (u32, u32, u32, u32) {
    let mut min_x = img.width();
    let mut min_y = img.height();
    let mut max_x = 0;
    let mut max_y = 0;

    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] > 0 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    (min_x, min_y, max_x, max_y)
}

fn alpha_centroid(img: &::image::RgbaImage) -> (f32, f32) {
    let mut weighted_x = 0.0_f64;
    let mut weighted_y = 0.0_f64;
    let mut total_alpha = 0.0_f64;

    for (x, y, pixel) in img.enumerate_pixels() {
        let alpha = f64::from(pixel[3]);
        if alpha > 0.0 {
            weighted_x += x as f64 * alpha;
            weighted_y += y as f64 * alpha;
            total_alpha += alpha;
        }
    }

    if total_alpha == 0.0 {
        return (img.width() as f32 / 2.0, img.height() as f32 / 2.0);
    }

    (
        (weighted_x / total_alpha) as f32,
        (weighted_y / total_alpha) as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let settings = Settings {
            cockroach_count: 3,
            show_notifications: false,
            ..Settings::default()
        };

        App {
            edit: settings.clone(),
            settings: settings.clone(),
            timer: Timer::new(settings.interval_minutes, settings.duration_seconds),
            tray: None,
            frames: Some(vec![SpriteFrame {
                handle: iced_image::Handle::from_rgba(1, 1, vec![255, 255, 255, 255]),
                width: 1.0,
                height: 1.0,
                body_anchor_x: 0.5,
                body_anchor_y: 0.5,
            }]),
            pending_break: false,
            settings_window: None,
            overlays: Vec::new(),
            anim_start: Instant::now(),
        }
    }

    #[test]
    fn reuses_overlay_windows_between_breaks() {
        let mut app = test_app();
        let screens = vec![platform::ScreenFrame {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        }];

        let _ = app.show_overlays(screens.clone());
        let first_ids: Vec<_> = app.overlays.iter().map(|ov| ov.id).collect();
        assert_eq!(app.overlays.len(), 1);
        assert!(app.any_active_overlay());

        let _ = app.close_overlays();
        assert_eq!(app.overlays.len(), 1);
        assert!(!app.any_active_overlay());
        assert!(app.overlays[0].cockroaches.is_empty());

        let _ = app.show_overlays(screens);
        let second_ids: Vec<_> = app.overlays.iter().map(|ov| ov.id).collect();
        assert_eq!(second_ids, first_ids);
        assert!(app.any_active_overlay());
        assert_eq!(app.overlays[0].cockroaches.len(), 3);
    }

    #[test]
    fn sprite_frames_are_capped_to_runtime_size() {
        let frames = load_sprite_frames(&constants::FRAME_BYTES);

        assert_eq!(frames.len(), constants::TOTAL_FRAMES);
        assert!(frames
            .iter()
            .all(|frame| frame.width <= MAX_SPRITE_FRAME_WIDTH as f32));
        assert!(frames.iter().all(|frame| frame.height > 0.0));
    }
}
