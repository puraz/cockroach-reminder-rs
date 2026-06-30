//! The transparent fullscreen overlay: one per display, each running its own set
//! of cockroaches (matching the per-window behavior of `overlayManager.js`, where
//! every window receives the full `count`).

use crate::cockroach::Cockroach;
use crate::constants::MAX_SPRITE_WIDTH;
use crate::Message;
use iced::mouse;
use iced::widget::canvas::{Frame, Geometry, Image, Program};
use iced::widget::image;
use iced::{Point, Rectangle, Renderer, Size, Theme, Vector};

/// State for a single display's overlay window.
pub struct Overlay {
    pub id: iced::window::Id,
    pub width: f32,
    pub height: f32,
    pub cockroaches: Vec<Cockroach>,
}

pub struct SpriteFrame {
    pub handle: image::Handle,
    pub width: f32,
    pub height: f32,
    pub body_anchor_x: f32,
    pub body_anchor_y: f32,
}

/// Canvas program that paints the cockroaches of one [`Overlay`].
pub struct OverlayCanvas<'a> {
    pub overlay: &'a Overlay,
    pub frames: &'a [SpriteFrame],
}

impl<'a> Program<Message> for OverlayCanvas<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        for roach in &self.overlay.cockroaches {
            if !roach.is_drawable() {
                continue;
            }
            let sprite = &self.frames[roach.cur_frame];
            let w = roach.el_width(bounds.width);
            // Scale maps sprite source pixels (resized to MAX_SPRITE_WIDTH) to display pixels.
            let scale = w / MAX_SPRITE_WIDTH as f32;

            frame.with_save(|f| {
                f.translate(Vector::new(roach.center_x, roach.center_y));
                f.rotate(roach.angle_deg().to_radians());
                let rect = Rectangle::new(
                    Point::new(
                        -sprite.body_anchor_x * scale,
                        -sprite.body_anchor_y * scale,
                    ),
                    Size::new(sprite.width * scale, sprite.height * scale),
                );
                f.draw_image(rect, Image::new(sprite.handle.clone()));
            });
        }

        vec![frame.into_geometry()]
    }
}
