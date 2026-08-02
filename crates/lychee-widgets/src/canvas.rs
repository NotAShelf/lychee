use iced::mouse::{self, Cursor, Interaction};
use iced::widget::canvas::{self, Canvas, Geometry};
use iced::widget::image::Handle;
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced_graphics::geometry::Image as GraphicsImage;

/// A complete view update. Offsets are screen pixels relative to the viewport
/// centre; keeping them in screen space makes dragging and zooming compose
/// without scale-dependent conversions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    PanTo(Point),
    ZoomTo { scale: f32, pan_offset: Point },
}

#[derive(Debug, Default)]
pub struct CanvasInteraction {
    dragging: Option<Drag>,
}

#[derive(Debug, Clone, Copy)]
struct Drag {
    start: Point,
    pan_offset: Point,
}

/// An image viewport with cursor-anchored wheel zoom and left-button panning.
///
/// The application owns the view state. The canvas only keeps enough transient
/// state to turn a drag into an absolute `PanTo` update.
#[derive(Debug)]
pub struct ImageCanvas {
    handle: Handle,
    image_size: Size,
    scale: f32,
    pan_offset: Point,
    min_scale: f32,
    max_scale: f32,
    zoom_step: f32,
}

impl ImageCanvas {
    pub fn new(
        handle: Handle,
        width: u32,
        height: u32,
        scale: f32,
        pan_offset: Point,
        min_scale: f32,
        zoom_step: f32,
    ) -> Self {
        Self {
            handle,
            image_size: Size::new(width as f32, height as f32),
            scale,
            pan_offset,
            min_scale,
            max_scale: 100.0,
            zoom_step,
        }
    }

    pub fn into_element(self) -> Element<'static, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Limits panning so that some of the image remains visible.
    ///
    /// This deliberately permits panning an image that fits the viewport. It
    /// matches imv's direct-manipulation behaviour and avoids a dead drag.
    pub fn clamp_pan(pan: Point, scale: f32, image_size: Size, viewport: Size) -> Point {
        let limit = |image: f32, viewport: f32| (image * scale + viewport) / 2.0;
        let x = limit(image_size.width, viewport.width);
        let y = limit(image_size.height, viewport.height);

        Point::new(pan.x.clamp(-x, x), pan.y.clamp(-y, y))
    }

    /// Returns the pan that leaves the image point under `cursor` unchanged
    /// when changing scale. `cursor` is local to the viewport.
    pub fn pan_for_zoom(
        pan: Point,
        old_scale: f32,
        new_scale: f32,
        cursor: Point,
        viewport: Size,
    ) -> Point {
        let factor = new_scale / old_scale;
        let from_center = Vector::new(
            cursor.x - viewport.width / 2.0,
            cursor.y - viewport.height / 2.0,
        );

        Point::new(
            factor * pan.x + (1.0 - factor) * from_center.x,
            factor * pan.y + (1.0 - factor) * from_center.y,
        )
    }

    fn local_cursor(cursor: Cursor, bounds: Rectangle) -> Option<Point> {
        cursor
            .position()
            .map(|point| Point::new(point.x - bounds.x, point.y - bounds.y))
    }
}

impl canvas::Program<Message> for ImageCanvas {
    type State = CanvasInteraction;

    fn update(
        &self,
        state: &mut CanvasInteraction,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let start = cursor.position_in(bounds)?;
                state.dragging = Some(Drag {
                    start,
                    pan_offset: self.pan_offset,
                });
                Some(canvas::Action::capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let drag = state.dragging?;
                let cursor = Self::local_cursor(cursor, bounds)?;
                let pan_offset = Self::clamp_pan(
                    Point::new(
                        drag.pan_offset.x + cursor.x - drag.start.x,
                        drag.pan_offset.y + cursor.y - drag.start.y,
                    ),
                    self.scale,
                    self.image_size,
                    bounds.size(),
                );
                Some(canvas::Action::publish(Message::PanTo(pan_offset)))
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging.take().is_some() {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let cursor = cursor.position_in(bounds)?;
                let ticks = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    // Trackpads report pixels. 60 px is deliberately one wheel
                    // notch, so both devices have comparable zoom sensitivity.
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 60.0,
                };
                if ticks == 0.0 {
                    return None;
                }

                let scale =
                    (self.scale * self.zoom_step.powf(ticks)).clamp(self.min_scale, self.max_scale);
                if (scale - self.scale).abs() < f32::EPSILON {
                    return None;
                }

                let pan_offset = Self::clamp_pan(
                    Self::pan_for_zoom(self.pan_offset, self.scale, scale, cursor, bounds.size()),
                    scale,
                    self.image_size,
                    bounds.size(),
                );
                Some(canvas::Action::publish(Message::ZoomTo {
                    scale,
                    pan_offset,
                }))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &CanvasInteraction,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // Do not cache this geometry: panning and zooming deliberately change
        // the transform on every pointer event.
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.with_save(|frame| {
            frame.translate(Vector::new(
                bounds.width / 2.0 + self.pan_offset.x,
                bounds.height / 2.0 + self.pan_offset.y,
            ));
            frame.scale(self.scale);
            frame.translate(Vector::new(
                -self.image_size.width / 2.0,
                -self.image_size.height / 2.0,
            ));
            frame.draw_image(
                Rectangle::new(Point::ORIGIN, self.image_size),
                GraphicsImage::new(&self.handle),
            );
        });
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &CanvasInteraction,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Interaction {
        if state.dragging.is_some() {
            Interaction::Grabbing
        } else {
            Interaction::Grab
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ImageCanvas;
    use iced::{Point, Size};

    #[test]
    fn zoom_keeps_the_cursor_anchor_fixed() {
        let viewport = Size::new(800.0, 600.0);
        let pan = Point::new(30.0, -20.0);
        let cursor = Point::new(650.0, 180.0);
        let new_pan = ImageCanvas::pan_for_zoom(pan, 1.0, 2.0, cursor, viewport);

        let image_point = Point::new(
            (cursor.x - viewport.width / 2.0 - pan.x) / 1.0,
            (cursor.y - viewport.height / 2.0 - pan.y) / 1.0,
        );
        let after = Point::new(
            viewport.width / 2.0 + new_pan.x + 2.0 * image_point.x,
            viewport.height / 2.0 + new_pan.y + 2.0 * image_point.y,
        );

        assert!((after.x - cursor.x).abs() < f32::EPSILON);
        assert!((after.y - cursor.y).abs() < f32::EPSILON);
    }

    #[test]
    fn a_fitting_image_can_still_be_panned() {
        assert_eq!(
            ImageCanvas::clamp_pan(
                Point::new(50.0, -50.0),
                1.0,
                Size::new(400.0, 300.0),
                Size::new(800.0, 600.0),
            ),
            Point::new(50.0, -50.0),
        );
    }

    #[test]
    fn pan_stops_before_the_image_leaves_the_viewport() {
        assert_eq!(
            ImageCanvas::clamp_pan(
                Point::new(2_000.0, -2_000.0),
                2.0,
                Size::new(800.0, 600.0),
                Size::new(800.0, 600.0),
            ),
            Point::new(1_200.0, -900.0),
        );
    }
}
