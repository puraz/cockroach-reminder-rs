//! Settings form UI for the Cockroach Reminder app.
//!
//! Extracted from `main.rs` to isolate the settings window's widget tree
//! from the app's core lifecycle. The single public function [`view`]
//! takes what it needs and returns an iced [`Element`].

use crate::config::Settings;
use crate::timer::Phase;
use crate::Message;

use iced::widget::{button, checkbox, column, container, row, scrollable, slider, text};
use iced::{Alignment, Color, Element, Length, Theme};

/// Render the settings window content.
pub fn view(edit: &Settings, phase: Phase, formatted: &str) -> Element<'static, Message> {
    let header = column![
        text("🪳 蟑螂提醒").size(24).color(Color::WHITE),
        text("定时休息，保护健康！")
            .size(13)
            .color(color(0x88, 0x88, 0x88)),
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .width(Length::Fill);

    let timer_section = section(
        "⏱ 计时器",
        column![
            slider_row(
                "休息间隔",
                slider(1..=120, edit.interval_minutes, Message::IntervalChanged).into(),
                format!("{} 分钟", edit.interval_minutes),
            ),
            slider_row(
                "显示时长",
                slider(3..=120, edit.duration_seconds, Message::DurationChanged).into(),
                format!("{} 秒", edit.duration_seconds),
            ),
            slider_row(
                "蟑螂数量",
                slider(1..=50, edit.cockroach_count, Message::CountChanged).into(),
                format!("{}", edit.cockroach_count),
            ),
        ]
        .spacing(14),
    );

    let fast_prob = (edit.fast_speed_probability * 100.0).round() as u32;
    let anim_section = section(
        "🎨 动画",
        column![
            slider_row(
                "蟑螂大小",
                slider(
                    10.0..=80.0,
                    edit.cockroach_size_percent,
                    Message::SizeChanged
                )
                .step(1.0f32)
                .into(),
                format!("{}%", edit.cockroach_size_percent.round() as u32),
            ),
            slider_row(
                "移动速度",
                slider(5.0..=50.0, edit.movement_percent, Message::SpeedChanged)
                    .step(0.5f32)
                    .into(),
                format!("{:.1}%", edit.movement_percent),
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
            checkbox(edit.auto_start)
                .label("启动应用时自动开启计时")
                .on_toggle(Message::AutoStartToggled),
            checkbox(edit.launch_at_login)
                .label("开机自启动")
                .on_toggle(Message::LaunchAtLoginToggled),
            checkbox(edit.show_notifications)
                .label("显示系统通知")
                .on_toggle(Message::ShowNotificationsToggled),
        ]
        .spacing(12),
    );

    let pause_label = if phase == Phase::Running {
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

    let status_text = status_line(phase, formatted);
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
        .style(|_: &Theme| container::Style {
            background: Some(color(0x1a, 0x1a, 0x2e).into()),
            text_color: Some(color(0xe0, 0xe0, 0xe0)),
            ..container::Style::default()
        })
        .into()
}

// --- Private helpers ---

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
        .align_y(Alignment::Center),
    ]
    .spacing(6)
    .into()
}

fn status_line(phase: Phase, formatted: &str) -> String {
    match phase {
        Phase::Running => format!("计时中 — 下次休息还有 {formatted}"),
        Phase::Break => format!("休息时间！还剩 {formatted}"),
        Phase::Paused => format!("已暂停 — 剩余 {formatted}"),
        Phase::Idle => "计时器已停止".to_string(),
    }
}
