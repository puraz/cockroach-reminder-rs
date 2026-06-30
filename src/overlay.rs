//! The transparent fullscreen overlay: one per display, each running its own set
//! of cockroaches (matching the per-window behavior of `overlayManager.js`, where
//! every window receives the full `count`).

use crate::cockroach::Cockroach;
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
    pub active: bool,
    pub cockroaches: Vec<Cockroach>,
}

#[derive(Clone)]
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
            let h = roach.el_height(bounds.width);
            let scale_x = w / 1920.0;
            let scale_y = h / 1080.0;

            frame.with_save(|f| {
                f.translate(Vector::new(roach.center_x, roach.center_y));
                f.rotate(roach.angle_deg().to_radians());
                let rect = Rectangle::new(
                    Point::new(
                        -sprite.body_anchor_x * scale_x,
                        -sprite.body_anchor_y * scale_y,
                    ),
                    Size::new(sprite.width * scale_x, sprite.height * scale_y),
                );
                f.draw_image(rect, Image::new(sprite.handle.clone()));
            });
        }

        vec![frame.into_geometry()]
    }
}
