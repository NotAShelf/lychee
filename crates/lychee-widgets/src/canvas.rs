use iced::mouse::{self, Cursor, Interaction};
use iced::widget::canvas::{self, Geometry};
use iced::widget::canvas::{Cache, Canvas};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced_graphics::geometry::Image as GraphicsImage;

use iced::Event;
use iced::widget::image::Handle;

/// Message types emitted by [`ImageCanvas`].
///
/// These are used to communicate user interactions to the application.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    /// Pan by the given delta (dx, dy) in screen pixels.
    Pan(f32, f32),
    /// Wheel zoom request with cursor and viewport center for zoom-around-cursor.
    Zoom {
        factor: f32,
        cursor: Point,
        viewport_center: Point,
    },
    /// Zoom in.
    ZoomIn,
    /// Zoom out.
    ZoomOut,
}

/// Interaction state for the canvas.
///
/// Tracks the current drag state for panning and visual zoom during drag.
#[derive(Debug, Clone)]
pub struct CanvasInteraction {
    /// Current visual pan offset during interaction.
    pan_offset: Point,
    /// Current visual zoom scale during interaction.
    zoom_scale: f32,
    /// Whether we're currently dragging.
    is_dragging: bool,
    /// Cursor position when drag started.
    drag_start: Point,
    /// Pan offset when drag started.
    initial_pan: Point,
}

impl Default for CanvasInteraction {
    fn default() -> Self {
        Self {
            pan_offset: Point::ORIGIN,
            zoom_scale: 1.0,
            is_dragging: false,
            drag_start: Point::ORIGIN,
            initial_pan: Point::ORIGIN,
        }
    }
}

/// A pan/zoom capable canvas widget for image viewing.
///
/// This widget renders an image and provides visual pan and zoom capabilities
/// through mouse interaction:
///
/// - **Mouse drag**: Pan the view
/// - **Scroll wheel**: Zoom in/out (publishes message for App to handle)
///
/// The widget is designed to be integrated with an App that manages the
/// authoritative state. Mouse events during drag update visual state for
/// smoothness, but publish messages to sync with App state when interaction ends.
#[derive(Debug)]
pub struct ImageCanvas {
    /// The image data handle.
    handle: Handle,
    /// Image dimensions in pixels.
    width: u32,
    height: u32,
    /// Authoritative zoom scale from App.
    zoom_scale: f32,
    /// Authoritative pan offset from App.
    pan_offset: Point,
    /// Minimum zoom scale.
    min_zoom: f32,
    /// Maximum zoom scale.
    max_zoom: f32,
    /// Zoom step multiplier per scroll tick.
    zoom_step: f32,
    /// Geometry cache for rendering.
    cache: Cache,
}

impl ImageCanvas {
    /// Create a new [`ImageCanvas`] from an image handle and dimensions.
    ///
    /// # Arguments
    /// * `handle` - The image handle
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `zoom_scale` - Initial zoom scale (1.0 = 100%)
    /// * `pan_offset` - Initial pan offset
    /// * `zoom_step` - Zoom multiplier per scroll tick (e.g., 1.1 for 10% steps)
    pub fn new(
        handle: Handle,
        width: u32,
        height: u32,
        zoom_scale: f32,
        pan_offset: Point,
        zoom_step: f32,
    ) -> Self {
        Self {
            handle,
            width,
            height,
            zoom_scale,
            pan_offset,
            min_zoom: 0.1,  // 10%
            max_zoom: 10.0, // 1000%
            zoom_step,
            cache: Cache::default(),
        }
    }

    /// Apply a zoom factor to the given pan offset, returning adjusted values.
    ///
    /// Used by App to calculate new pan when zoom changes.
    pub fn adjust_pan_for_zoom(pan: Point, old_scale: f32, new_scale: f32, cursor: Point) -> Point {
        if (new_scale - old_scale).abs() < f32::EPSILON {
            return pan;
        }

        let scale_delta = new_scale / old_scale;
        Point::new(
            cursor.x - (cursor.x - pan.x) * scale_delta,
            cursor.y - (cursor.y - pan.y) * scale_delta,
        )
    }

    /// Clamp pan offset based on zoom level and viewport size.
    ///
    /// Returns the clamped pan offset that keeps the image reasonably visible.
    pub fn clamp_pan(pan: Point, zoom_scale: f32, viewport: Size) -> Point {
        // At scale 1.0, no panning needed (image fills viewport)
        // At scale > 1.0, allow panning to see offscreen parts
        // At scale < 1.0, limit to keep image centered
        let max_pan_x = if zoom_scale > 1.0 {
            (zoom_scale - 1.0) * viewport.width / (2.0 * zoom_scale)
        } else {
            (1.0 - zoom_scale) * viewport.width / 2.0
        };
        let max_pan_y = if zoom_scale > 1.0 {
            (zoom_scale - 1.0) * viewport.height / (2.0 * zoom_scale)
        } else {
            (1.0 - zoom_scale) * viewport.height / 2.0
        };

        Point::new(
            pan.x.clamp(-max_pan_x, max_pan_x),
            pan.y.clamp(-max_pan_y, max_pan_y),
        )
    }

    /// Convert to an Iced element for embedding in UI.
    pub fn into_element(self) -> Element<'static, Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl canvas::Program<Message> for ImageCanvas {
    type State = CanvasInteraction;

    fn update(
        &self,
        interaction: &mut CanvasInteraction,
        event: &Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<canvas::Action<Message>> {
        let cursor_position = cursor.position_in(bounds)?;

        match event {
            Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    // Start panning - initialize interaction state from App state
                    interaction.pan_offset = self.pan_offset;
                    interaction.zoom_scale = self.zoom_scale;
                    interaction.is_dragging = true;
                    interaction.drag_start = cursor_position;
                    interaction.initial_pan = self.pan_offset;
                    Some(canvas::Action::capture())
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    if interaction.is_dragging {
                        interaction.is_dragging = false;
                        // Publish final pan to sync with App
                        let dx = interaction.pan_offset.x - self.pan_offset.x;
                        let dy = interaction.pan_offset.y - self.pan_offset.y;
                        if dx.abs() > 0.1 || dy.abs() > 0.1 {
                            return Some(canvas::Action::publish(Message::Pan(dx, dy)));
                        }
                    }
                    None
                }
                mouse::Event::CursorMoved { .. } => {
                    if interaction.is_dragging {
                        // Calculate delta from drag start position
                        let dx = cursor_position.x - interaction.drag_start.x;
                        let dy = cursor_position.y - interaction.drag_start.y;

                        // Update visual pan offset from initial pan
                        interaction.pan_offset = Point::new(
                            interaction.initial_pan.x + dx / interaction.zoom_scale,
                            interaction.initial_pan.y + dy / interaction.zoom_scale,
                        );

                        return Some(canvas::Action::request_redraw());
                    }
                    None
                }
                mouse::Event::WheelScrolled { delta } => {
                    // Publish zoom message for App to handle
                    // This ensures App state stays in sync
                    let factor = match delta {
                        mouse::ScrollDelta::Lines { y, .. }
                        | mouse::ScrollDelta::Pixels { y, .. } => {
                            if *y > 0.0 {
                                self.zoom_step
                            } else if *y < 0.0 {
                                1.0 / self.zoom_step
                            } else {
                                1.0
                            }
                        }
                    };

                    if (factor - 1.0).abs() > f32::EPSILON {
                        // Clamp to valid range
                        let new_scale =
                            (self.zoom_scale * factor).clamp(self.min_zoom, self.max_zoom);
                        if (new_scale - self.zoom_scale).abs() > f32::EPSILON {
                            // Publish actual factor that produced the clamped value
                            let clamped_factor = new_scale / self.zoom_scale;
                            // Calculate viewport center from bounds
                            let viewport_center =
                                Point::new(bounds.width / 2.0, bounds.height / 2.0);
                            // Pass cursor position and viewport center for zoom-around-cursor
                            return Some(canvas::Action::publish(Message::Zoom {
                                factor: clamped_factor,
                                cursor: cursor_position,
                                viewport_center,
                            }));
                        }
                    }
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn draw(
        &self,
        interaction: &CanvasInteraction,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // Use interaction state if dragging, otherwise use App state
        let pan_offset = if interaction.is_dragging {
            interaction.pan_offset
        } else {
            self.pan_offset
        };

        let zoom_scale = if interaction.is_dragging {
            interaction.zoom_scale
        } else {
            self.zoom_scale
        };

        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        // Draw the image using the cache
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            frame.with_save(|frame| {
                // Move to center of viewport
                frame.translate(Vector::new(center.x, center.y));

                // Apply zoom (around center)
                frame.scale(zoom_scale);

                // Apply pan (already in zoomed coordinates)
                frame.translate(Vector::new(pan_offset.x, pan_offset.y));

                // Center image at origin
                frame.translate(Vector::new(
                    -(self.width as f32) / 2.0,
                    -(self.height as f32) / 2.0,
                ));

                // Draw the image
                // Note: We draw at origin since we already translated to center
                let image = GraphicsImage::new(&self.handle);
                frame.draw_image(
                    Rectangle::new(
                        Point::ORIGIN,
                        Size::new(self.width as f32, self.height as f32),
                    ),
                    image,
                );
            });
        });

        vec![geometry]
    }

    fn mouse_interaction(
        &self,
        interaction: &CanvasInteraction,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Interaction {
        if interaction.is_dragging {
            Interaction::Grabbing
        } else {
            Interaction::Grab
        }
    }
}
