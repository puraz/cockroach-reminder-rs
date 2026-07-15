//! Hallmark · genre: modern-minimal · macrostructure: Workbench · theme: Coral
//! tone: premium minimal · enrichment: none · designed-as-app: yes
//! Hallmark · pre-emit critique: P5 H5 E4 S5 R5 V5 · contrast: pass (40-41)
//!
//! Settings form UI for the Cockroach Reminder app. The view intentionally
//! keeps timing status ahead of configuration, so the active routine remains
//! the visual anchor while controls stay close at hand.

use crate::config::Settings;
use crate::timer::Phase;
use crate::Message;

use iced::widget::{button, checkbox, column, container, row, scrollable, slider, text};
use iced::{Alignment, Border, Color, Element, Length, Theme};

/// Render the settings window content.
pub fn view(settings: &Settings, phase: Phase, formatted: &str) -> Element<'static, Message> {
    let header = column![
        text("COCKROACH REMINDER").size(12).color(muted()),
        text("让休息准时发生").size(29).color(ink()),
    ]
    .spacing(7)
    .width(Length::Fill);

    let timer_section = section(
        column![
            slider_row(
                "休息间隔",
                slider(1..=120, settings.interval_minutes, Message::IntervalChanged)
                    .style(slider_style)
                    .into(),
                format!("{} 分钟", settings.interval_minutes),
            ),
            slider_row(
                "显示时长",
                slider(3..=120, settings.duration_seconds, Message::DurationChanged)
                    .style(slider_style)
                    .into(),
                format!("{} 秒", settings.duration_seconds),
            ),
            slider_row(
                "蟑螂数量",
                slider(1..=50, settings.cockroach_count, Message::CountChanged)
                    .style(slider_style)
                    .into(),
                format!("{} 只", settings.cockroach_count),
            ),
        ]
        .spacing(14),
    );

    let fast_prob = (settings.fast_speed_probability * 100.0).round() as u32;
    let animation_section = section(
        column![
            slider_row(
                "蟑螂大小",
                slider(
                    10.0..=80.0,
                    settings.cockroach_size_percent,
                    Message::SizeChanged,
                )
                .step(1.0f32)
                .style(slider_style)
                .into(),
                format!("{}%", settings.cockroach_size_percent.round() as u32),
            ),
            slider_row(
                "移动速度",
                slider(5.0..=50.0, settings.movement_percent, Message::SpeedChanged)
                    .step(0.5f32)
                    .style(slider_style)
                    .into(),
                format!("{:.1}%", settings.movement_percent),
            ),
            slider_row(
                "快速蟑螂概率",
                slider(0..=100, fast_prob, Message::FastProbChanged)
                    .step(5u32)
                    .style(slider_style)
                    .into(),
                format!("{}%", fast_prob),
            ),
        ]
        .spacing(14),
    );

    let behavior_section = section(
        column![
            checkbox(settings.auto_start)
                .label("启动应用时自动开启计时")
                .on_toggle(Message::AutoStartToggled)
                .style(checkbox_style),
            checkbox(settings.launch_at_login)
                .label("开机自启动")
                .on_toggle(Message::LaunchAtLoginToggled)
                .style(checkbox_style),
            checkbox(settings.show_notifications)
                .label("显示系统通知")
                .on_toggle(Message::ShowNotificationsToggled)
                .style(checkbox_style),
        ]
        .spacing(12),
    );

    let pause_label = if phase == Phase::Running {
        "暂停计时"
    } else {
        "继续计时"
    };

    let actions = row![
        button(text("立即休息").center().width(Length::Fill))
            .on_press(Message::TestBreak)
            .style(secondary_button_style)
            .width(Length::Fill)
            .padding([12, 16]),
        button(text(pause_label).center().width(Length::Fill))
            .on_press(Message::TogglePause)
            .style(secondary_button_style)
            .width(Length::Fill)
            .padding([12, 16]),
    ]
    .spacing(12);

    let content = column![
        header,
        status_card(phase, formatted),
        timer_section,
        animation_section,
        behavior_section,
        actions,
    ]
    .spacing(14)
    .max_width(440)
    .padding([24, 10]);

    let scroll = scrollable(
        container(content)
            .center_x(Length::Fill)
            .width(Length::Fill),
    );

    container(scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_: &Theme| container::Style {
            background: Some(canvas().into()),
            text_color: Some(ink()),
            ..container::Style::default()
        })
        .into()
}

fn section<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(body.into())
        .padding(18)
        .width(Length::Fill)
        .style(panel_style)
        .into()
}

fn slider_row<'a>(
    label: &'a str,
    slider: Element<'a, Message>,
    value: String,
) -> Element<'a, Message> {
    column![
        row![
            text(label).size(14).color(ink()).width(Length::Fill),
            value_badge(value),
        ]
        .align_y(Alignment::Center),
        slider,
    ]
    .spacing(9)
    .into()
}

fn status_card(phase: Phase, formatted: &str) -> Element<'static, Message> {
    let phase_text = match phase {
        Phase::Running => "计时中",
        Phase::Break => "休息中",
        Phase::Paused => "已暂停",
        Phase::Idle => "未开始",
    };

    container(
        row![
            column![text(status_line(phase, formatted)).size(17).color(ink()),].width(Length::Fill),
            phase_badge(phase_text),
        ]
        .align_y(Alignment::Center),
    )
    .padding([15, 18])
    .width(Length::Fill)
    .style(status_style)
    .into()
}

fn phase_badge(label: &str) -> Element<'_, Message> {
    container(text(label).size(13).color(canvas()))
        .padding([6, 10])
        .style(|_: &Theme| container::Style {
            background: Some(accent().into()),
            border: Border {
                radius: 7.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn value_badge(value: String) -> Element<'static, Message> {
    container(text(value).size(13).color(accent()))
        .padding([5, 9])
        .style(value_badge_style)
        .into()
}

fn panel_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(surface().into()),
        border: Border {
            color: rule(),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(ink()),
        ..container::Style::default()
    }
}

fn status_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(surface_raised().into()),
        border: Border {
            color: accent_dim(),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(ink()),
        ..container::Style::default()
    }
}

fn value_badge_style(_: &Theme) -> container::Style {
    container::Style {
        background: Some(accent_wash().into()),
        border: Border {
            color: accent_dim(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

fn secondary_button_style(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => surface_raised(),
        button::Status::Pressed => accent_wash(),
        button::Status::Active | button::Status::Disabled => surface(),
    };

    button::Style {
        background: Some(background.into()),
        text_color: ink(),
        border: Border {
            color: rule(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..button::Style::default()
    }
}

fn slider_style(_: &Theme, status: slider::Status) -> slider::Style {
    let active = match status {
        slider::Status::Active => accent(),
        slider::Status::Hovered | slider::Status::Dragged => accent_hover(),
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (active.into(), rule().into()),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Border::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: active.into(),
            border_width: 2.0,
            border_color: canvas(),
        },
    }
}

fn checkbox_style(_: &Theme, status: checkbox::Status) -> checkbox::Style {
    let (is_checked, is_hovered, is_disabled) = match status {
        checkbox::Status::Active { is_checked } => (is_checked, false, false),
        checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
        checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
    };
    let background = if is_checked {
        accent()
    } else {
        surface_raised()
    };
    let border_color = if is_hovered { accent_dim() } else { rule() };

    checkbox::Style {
        background: background.into(),
        icon_color: canvas(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 4.0.into(),
        },
        text_color: Some(if is_disabled { muted() } else { ink() }),
    }
}

fn status_line(phase: Phase, formatted: &str) -> String {
    match phase {
        Phase::Running => format!("下次休息还有 {formatted}"),
        Phase::Break => format!("休息时间，还剩 {formatted}"),
        Phase::Paused => format!("计时已暂停，剩余 {formatted}"),
        Phase::Idle => "计时器尚未开始".to_string(),
    }
}

// Native equivalents of the design tokens for this Iced surface.
fn color(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn canvas() -> Color {
    color(0x0c, 0x0c, 0x0e)
}

fn surface() -> Color {
    color(0x15, 0x15, 0x18)
}

fn surface_raised() -> Color {
    color(0x1d, 0x1d, 0x22)
}

fn ink() -> Color {
    color(0xf4, 0xf0, 0xe8)
}

fn muted() -> Color {
    color(0xa5, 0xa3, 0xa3)
}

fn rule() -> Color {
    color(0x35, 0x35, 0x3a)
}

fn accent() -> Color {
    color(0xc4, 0xa3, 0x6a)
}

fn accent_hover() -> Color {
    color(0xd7, 0xb9, 0x78)
}

fn accent_dim() -> Color {
    color(0x7c, 0x65, 0x42)
}

fn accent_wash() -> Color {
    color(0x2b, 0x26, 0x20)
}
