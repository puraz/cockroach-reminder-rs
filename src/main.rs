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
mod timer;
mod tray;

use cockroach::{AnimConfig, Cockroach};
use config::Settings;
use overlay::{Overlay, OverlayCanvas, SpriteFrame};
use timer::{Phase, Timer, Transition};
use tray::{Tray, TrayCommand};

use iced::widget::image;
use iced::widget::{
    button, canvas, checkbox, column, container, row, scrollable, slider, text, Space,
};
use iced::{window, Color, Element, Length, Subscription, Task, Theme};
use std::time::{Duration, Instant};

fn main() -> iced::Result {
    iced::daemon(App::new, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(App::theme)
        .style(App::style)
        .default_font(iced::Font::with_name("PingFang SC"))
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Anim(Instant),
    PollTray,

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
    frames: Vec<SpriteFrame>,

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

        let frames = load_sprite_frames(&constants::FRAME_BYTES);

        // Auto-start the timer if configured.
        if settings.auto_start {
            timer.start();
        }

        let mut app = App {
            edit: settings.clone(),
            settings,
            timer,
            tray: None,
            frames,
            settings_window: None,
            overlays: Vec::new(),
            anim_start: Instant::now(),
        };

        let (status, toggle_label, toggle_enabled, tooltip) = app.tray_labels();
        app.tray = Tray::new(&status, &toggle_label, toggle_enabled, &tooltip);

        (app, Task::none())
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
        if !self.overlays.is_empty() {
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
        } else if let Some(ov) = self.overlays.iter().find(|o| o.id == window) {
            canvas(OverlayCanvas {
                overlay: ov,
                frames: &self.frames,
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

    /// Open one transparent overlay per display and seed it with `cockroach_count` roaches.
    fn spawn_overlays(&mut self) -> Task<Message> {
        let mut tasks = self.close_overlays_tasks();

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

        self.anim_start = Instant::now();
        let cfg = self.anim_config();
        let count = self.settings.cockroach_count;
        let mut rng = rand::thread_rng();

        for (i, sf) in screens.into_iter().enumerate() {
            let w = sf.width as f32;
            let h = sf.height as f32;

            let cockroaches = (0..count)
                .map(|_| Cockroach::new(&mut rng, cfg, w, h))
                .collect();

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

        Task::batch(tasks)
    }

    fn close_overlays(&mut self) -> Task<Message> {
        Task::batch(self.close_overlays_tasks())
    }

    fn close_overlays_tasks(&mut self) -> Vec<Task<Message>> {
        let tasks = self
            .overlays
            .iter()
            .map(|ov| window::close::<Message>(ov.id))
            .collect();
        self.overlays.clear();
        tasks
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
            icon: window::icon::from_file_data(constants::APP_ICON_BYTES, None).ok(),
            ..Default::default()
        });
        self.settings_window = Some(id);
        platform::bring_to_front();
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
        let s = &self.edit;

        let header = column![
            text("🪳 蟑螂提醒").size(24).color(Color::WHITE),
            text("定时休息，保护健康！")
                .size(13)
                .color(color(0x88, 0x88, 0x88)),
        ]
        .spacing(4)
        .align_x(iced::Alignment::Center)
        .width(Length::Fill);

        let timer_section = section(
            "⏱ 计时器",
            column![
                slider_row(
                    "休息间隔",
                    slider(1..=120, s.interval_minutes, Message::IntervalChanged).into(),
                    format!("{} 分钟", s.interval_minutes),
                ),
                slider_row(
                    "显示时长",
                    slider(3..=120, s.duration_seconds, Message::DurationChanged).into(),
                    format!("{} 秒", s.duration_seconds),
                ),
                slider_row(
                    "蟑螂数量",
                    slider(1..=50, s.cockroach_count, Message::CountChanged).into(),
                    format!("{}", s.cockroach_count),
                ),
            ]
            .spacing(14),
        );

        let fast_prob = (s.fast_speed_probability * 100.0).round() as u32;
        let anim_section = section(
            "🎨 动画",
            column![
                slider_row(
                    "蟑螂大小",
                    slider(10.0..=80.0, s.cockroach_size_percent, Message::SizeChanged)
                        .step(1.0f32)
                        .into(),
                    format!("{}%", s.cockroach_size_percent.round() as u32),
                ),
                slider_row(
                    "移动速度",
                    slider(5.0..=50.0, s.movement_percent, Message::SpeedChanged)
                        .step(0.5f32)
                        .into(),
                    format!("{:.1}%", s.movement_percent),
                ),
                slider_row(
                    "快速蟑螂概率",
                    slider(0..=100, fast_prob, Message::FastProbChanged)
                        .step(5u32)
                        .into(),
                    format!("{}%", fast_prob),
                ),
            ]
            .spacing(14),
        );

        let behavior_section = section(
            "⚙ 行为",
            column![
                checkbox(s.auto_start)
                    .label("启动应用时自动开启计时")
                    .on_toggle(Message::AutoStartToggled),
                checkbox(s.launch_at_login)
                    .label("开机自启动")
                    .on_toggle(Message::LaunchAtLoginToggled),
                checkbox(s.show_notifications)
                    .label("显示系统通知")
                    .on_toggle(Message::ShowNotificationsToggled),
            ]
            .spacing(12),
        );

        let pause_label = if self.timer.phase == Phase::Running {
            "⏸ 暂停计时"
        } else {
            "▶ 恢复计时"
        };

        let actions = row![
            button(text("🪳 立即休息 (召唤蟑螂)").center())
                .on_press(Message::TestBreak)
                .style(button::danger)
                .width(Length::Fill),
            button(text(pause_label).center())
                .on_press(Message::TogglePause)
                .style(button::secondary)
                .width(Length::Fill),
        ]
        .spacing(10);

        let status_text = self.status_line();
        let status = container(text(status_text).size(14))
            .padding([10, 16])
            .width(Length::Fill)
            .style(section_style);

        let save = button(text("保存设置").center().size(15).width(Length::Fill))
            .on_press(Message::SaveSettings)
            .style(button::primary)
            .width(Length::Fill)
            .padding(12);

        let content = column![
            header,
            timer_section,
            anim_section,
            behavior_section,
            actions,
            status,
            save,
        ]
        .spacing(16)
        .max_width(520)
        .padding(24);

        let scroll = scrollable(
            container(content)
                .center_x(Length::Fill)
                .width(Length::Fill),
        );

        container(scroll)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(color(0x1a, 0x1a, 0x2e).into()),
                text_color: Some(color(0xe0, 0xe0, 0xe0)),
                ..container::Style::default()
            })
            .into()
    }

    fn status_line(&self) -> String {
        let f = self.timer.formatted();
        match self.timer.phase {
            Phase::Running => format!("计时中 — 下次休息还有 {f}"),
            Phase::Break => format!("休息时间！还剩 {f}"),
            Phase::Paused => format!("已暂停 — 剩余 {f}"),
            Phase::Idle => "计时器已停止".to_string(),
        }
    }
}

// --- Small UI helpers ---

fn color(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn section_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(color(0x16, 0x21, 0x3e).into()),
        border: iced::Border {
            color: color(0x0f, 0x34, 0x60),
            width: 1.0,
            radius: 12.0.into(),
        },
        text_color: Some(color(0xe0, 0xe0, 0xe0)),
        ..container::Style::default()
    }
}

fn section<'a>(title: &'a str, body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        column![
            text(title).size(15).color(color(0xe9, 0x45, 0x60)),
            body.into(),
        ]
        .spacing(14),
    )
    .padding([18, 20])
    .width(Length::Fill)
    .style(section_style)
    .into()
}

fn slider_row<'a>(
    label: &'a str,
    slider: Element<'a, Message>,
    value: String,
) -> Element<'a, Message> {
    column![
        text(label).size(13).color(color(0xaa, 0xaa, 0xaa)),
        row![
            slider,
            text(value)
                .size(14)
                .color(color(0xe9, 0x45, 0x60))
                .width(Length::Fixed(64.0)),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    ]
    .spacing(6)
    .into()
}

fn notify(title: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .show();
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
            let crop = ::image::imageops::crop_imm(img, global_min_x, global_min_y, crop_w, crop_h)
                .to_image();

            SpriteFrame {
                handle: image::Handle::from_rgba(crop_w, crop_h, crop.into_raw()),
                width: crop_w as f32,
                height: crop_h as f32,
                body_anchor_x: body_x - global_min_x as f32,
                body_anchor_y: body_y - global_min_y as f32,
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
