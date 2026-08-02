use std::collections::HashMap;

use iced::Point;
use iced::event;
use iced::{
    Element, Length, Subscription, Task, keyboard,
    time::{self, milliseconds},
    widget::{column, row},
    window,
};
use image::GenericImageView;
use lychee_cli::{Args, Parse};
use lychee_img::collect_paths;
use lychee_widgets::ImageCanvas;

use crate::keybindings::handle_key_event;

type ImageCacheKey = (usize, u16, bool, bool);
type ImageCacheValue = (iced::widget::image::Handle, u32, u32);

const STATUS_BAR_HEIGHT: f32 = 36.0;

#[derive(Debug, Clone)]
pub enum Message {
    Next,
    Prev,
    GoToFirst,
    GoToLast,
    ZoomIn,
    ZoomOut,
    ActualSize,
    FitWindow,
    ResetView,
    Pan(i32, i32),
    CanvasPan(Point),
    CanvasZoom { scale: f32, pan_offset: Point },
    RotateCW,
    RotateCCW,
    FlipHorizontal,
    FlipVertical,
    ResetTransform,
    ToggleFullscreen,
    ToggleSlideshow,
    SlideshowTick,
    AdjustSlideshowDelay(i64),
    WindowResized(u32, u32),
    Close,
}

pub struct App {
    window_id: window::Id,
    paths: Vec<String>,
    current: usize,
    // Cache key: (index, rotation_degrees, flip_h, flip_v)
    // Cache value: (image_handle, width, height)
    // NOTE: zoom is visual only (canvas transform), not in cache key
    image_cache: HashMap<ImageCacheKey, ImageCacheValue>,
    fullscreen: bool,
    slideshow_active: bool,
    slideshow_interval: u64,
    // Zoom state
    scale: f32,
    min_scale: f32,
    max_scale: f32,
    scale_step: f32,
    zoom_percent: u32,
    pan_offset: (f32, f32),
    view_locked: bool,
    // Transform state
    rotation: u16,
    flip_h: bool,
    flip_v: bool,
    // Window dimensions for fit-to-window calculation
    window_size: (u32, u32),
    // Original image dimensions for fit-to-window calculation
    current_image_size: (u32, u32),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let args = Args::parse();
        let paths = collect_paths(args.paths);
        let window_id = window::Id::unique();

        let mut app = Self {
            window_id,
            paths,
            current: 0,
            image_cache: HashMap::new(),
            fullscreen: args.fullscreen,
            slideshow_active: false,
            slideshow_interval: args.slideshow,
            // Zoom state
            scale: 1.0,
            min_scale: 0.1,
            max_scale: 100.0,
            scale_step: 0.04,
            zoom_percent: 100,
            pan_offset: (0.0, 0.0),
            view_locked: false,
            // Transform state
            rotation: 0,
            flip_h: false,
            flip_v: false,
            // Window and image dimensions
            window_size: (800, 600), // Default, will be updated by resize events
            current_image_size: (0, 0),
        };

        // Preload initial images into cache
        app.preload_adjacent();
        app.fit_view();

        let task = if args.fullscreen {
            window::maximize(window_id, true)
        } else {
            Task::none()
        };

        (app, task)
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.paths.is_empty() {
            return iced::widget::text("No images found.")
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        let image_content: Element<'_, Message> =
            match self
                .image_cache
                .get(&(self.current, self.rotation, self.flip_h, self.flip_v))
            {
                Some((handle, width, height)) => {
                    // 4% exponential steps match imv's controllable zoom.
                    let zoom_step = 1.0 + self.scale_step;
                    let canvas = ImageCanvas::new(
                        handle.clone(),
                        *width,
                        *height,
                        self.scale,
                        Point::new(self.pan_offset.0, self.pan_offset.1),
                        self.minimum_scale(),
                        zoom_step,
                    );

                    canvas.into_element().map(|msg| match msg {
                        lychee_widgets::Message::PanTo(pan_offset) => {
                            Message::CanvasPan(pan_offset)
                        }
                        lychee_widgets::Message::ZoomTo { scale, pan_offset } => {
                            Message::CanvasZoom { scale, pan_offset }
                        }
                    })
                }
                None => iced::widget::text("Loading...").into(),
            };

        // Main layout: image on top, statusbar at bottom
        let statusbar: Element<'_, Message> = self.render_statusbar();

        column![image_content, statusbar].into()
    }

    fn render_statusbar(&self) -> Element<'_, Message> {
        let zoom_text = format!("{:.*}%", 0, self.scale * 100.0);
        let index_text = format!("{}/{}", self.current + 1, self.paths.len());

        row![
            iced::widget::text(zoom_text).size(14),
            iced::widget::text(index_text).size(14),
        ]
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fixed(STATUS_BAR_HEIGHT))
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let hotkey = keyboard::listen().filter_map(|event| {
            if let keyboard::Event::KeyPressed { key, modifiers, .. } = event {
                handle_key_event(
                    &key,
                    modifiers.control(),
                    modifiers.shift(),
                    modifiers.is_empty(),
                )
            } else {
                None
            }
        });

        let close = window::close_events().map(|_| Message::Close);
        let slideshow = time::every(milliseconds(self.slideshow_interval * 1000))
            .map(|_| Message::SlideshowTick);
        let resize = event::listen().filter_map(|event| {
            if let iced::Event::Window(window::Event::Resized(size)) = event {
                Some(Message::WindowResized(
                    size.width as u32,
                    size.height as u32,
                ))
            } else {
                None
            }
        });

        Subscription::batch(vec![hotkey, close, slideshow, resize])
    }

    fn preload_adjacent(&mut self) {
        for offset in [0isize, -1, 1] {
            let Some(idx) = self.current.checked_add_signed(offset) else {
                continue;
            };
            let cache_key = (idx, self.rotation, self.flip_h, self.flip_v);
            if idx >= self.paths.len() {
                continue;
            }

            if let Some((_, width, height)) = self.image_cache.get(&cache_key) {
                if idx == self.current {
                    self.current_image_size = (*width, *height);
                }
            } else {
                // Extract path to avoid borrow conflict
                let path = self.paths[idx].clone();
                if let Some(handle_data) = self.load_image(&path) {
                    if idx == self.current {
                        self.current_image_size = (handle_data.1, handle_data.2);
                    }
                    self.image_cache.insert(cache_key, handle_data);
                }
            }
        }
    }

    fn load_image(&self, path: &str) -> Option<(iced::widget::image::Handle, u32, u32)> {
        let mut img = ::image::open(path).ok()?;

        // Apply rotation
        match self.rotation {
            90 => img = img.rotate90(),
            180 => img = img.rotate180(),
            270 => img = img.rotate270(),
            _ => {}
        }

        // Apply flips
        if self.flip_h {
            img = img.fliph();
        }
        if self.flip_v {
            img = img.flipv();
        }

        // Load at native resolution - zoom is purely visual via canvas transform
        let (width, height) = img.dimensions();
        let raw = img.to_rgba8().into_raw();
        let handle = iced::widget::image::Handle::from_rgba(width, height, raw);
        Some((handle, width, height))
    }

    fn invalidate_cache(&mut self) {
        // Invalidate current and adjacent images when transform changes. Zoom is actually
        // visual only, so it does not invalidate cache.
        for offset in [0isize, -1, 1] {
            let Some(idx) = self.current.checked_add_signed(offset) else {
                continue;
            };
            let key = (idx, self.rotation, self.flip_h, self.flip_v);
            self.image_cache.remove(&key);
        }
        self.preload_adjacent();
    }

    /// Ensure the current image is loaded into cache.
    /// We'll load it on-demand if missing.
    fn ensure_current_loaded(&mut self) {
        let key = (self.current, self.rotation, self.flip_h, self.flip_v);
        if !self.image_cache.contains_key(&key) {
            // Extract path to avoid borrow conflict
            let path = self.paths[self.current].clone();
            if let Some(handle_data) = self.load_image(&path) {
                self.current_image_size = (handle_data.1, handle_data.2);
                self.image_cache.insert(key, handle_data);
            }
        }
    }

    fn viewport_size(&self) -> iced::Size {
        iced::Size::new(
            self.window_size.0 as f32,
            (self.window_size.1 as f32 - STATUS_BAR_HEIGHT).max(0.0),
        )
    }

    fn fit_scale(&self) -> Option<f32> {
        let (image_width, image_height) = self.current_image_size;
        let viewport = self.viewport_size();
        if image_width == 0 || image_height == 0 || viewport.width == 0.0 || viewport.height == 0.0
        {
            return None;
        }

        Some(
            (viewport.width / image_width as f32)
                .min(viewport.height / image_height as f32)
                .min(self.max_scale),
        )
    }

    fn minimum_scale(&self) -> f32 {
        self.fit_scale()
            .unwrap_or(self.min_scale)
            .min(self.min_scale)
    }

    fn clamp_current_pan(&mut self) {
        let pan = ImageCanvas::clamp_pan(
            Point::new(self.pan_offset.0, self.pan_offset.1),
            self.scale,
            iced::Size::new(
                self.current_image_size.0 as f32,
                self.current_image_size.1 as f32,
            ),
            self.viewport_size(),
        );
        self.pan_offset = (pan.x, pan.y);
    }

    fn set_scale(&mut self, scale: f32) {
        self.scale = scale.clamp(self.minimum_scale(), self.max_scale);
        self.zoom_percent = (self.scale * 100.0).round() as u32;
        self.clamp_current_pan();
    }

    fn fit_view(&mut self) {
        let Some(scale) = self.fit_scale() else {
            return;
        };

        self.scale = scale;
        self.zoom_percent = (scale * 100.0).round() as u32;
        self.pan_offset = (0.0, 0.0);
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if self.paths.is_empty() {
            return match message {
                Message::Close => iced::exit(),
                _ => Task::none(),
            };
        }

        // Ensure current image is loaded; this handles edge cases where
        // cache miss occurred and preload hasn't triggered yet
        self.ensure_current_loaded();

        match message {
            Message::Next => {
                if self.current < self.paths.len() - 1 {
                    self.current += 1;
                    self.preload_adjacent();
                    if !self.view_locked {
                        self.fit_view();
                    }
                }
                Task::none()
            }
            Message::Prev => {
                if self.current > 0 {
                    self.current -= 1;
                    self.preload_adjacent();
                    if !self.view_locked {
                        self.fit_view();
                    }
                }
                Task::none()
            }
            Message::GoToFirst => {
                if !self.paths.is_empty() {
                    self.current = 0;
                    self.preload_adjacent();
                    if !self.view_locked {
                        self.fit_view();
                    }
                }
                Task::none()
            }
            Message::GoToLast => {
                if !self.paths.is_empty() {
                    self.current = self.paths.len() - 1;
                    self.preload_adjacent();
                    if !self.view_locked {
                        self.fit_view();
                    }
                }
                Task::none()
            }
            Message::ZoomIn => {
                // Zoom is visual only via canvas transform - no cache invalidation
                self.set_scale(self.scale * (1.0 + self.scale_step));
                self.view_locked = true;
                // No cache invalidation - image data unchanged
                Task::none()
            }
            Message::ZoomOut => {
                // Zoom is visual only via canvas transform - no cache invalidation
                self.set_scale(self.scale / (1.0 + self.scale_step));
                self.view_locked = true;
                // No cache invalidation - image data unchanged
                Task::none()
            }
            Message::ActualSize => {
                self.scale = 1.0;
                self.zoom_percent = 100;
                self.pan_offset = (0.0, 0.0);
                self.view_locked = true;
                Task::none()
            }
            Message::FitWindow | Message::ResetView => {
                self.view_locked = false;
                self.fit_view();
                Task::none()
            }
            Message::WindowResized(width, height) => {
                self.window_size = (width, height);
                if !self.view_locked {
                    self.fit_view();
                } else {
                    self.clamp_current_pan();
                }
                Task::none()
            }
            Message::Pan(dx, dy) => {
                // Update visual pan offset from canvas drag
                // Canvas sends raw screen pixel delta; keyboard sends 50 units per press
                // Use raw values directly for natural mouse feel
                self.pan_offset.0 += dx as f32;
                self.pan_offset.1 += dy as f32;
                self.clamp_current_pan();
                self.view_locked = true;
                Task::none()
            }
            Message::CanvasPan(pan_offset) => {
                self.pan_offset = (pan_offset.x, pan_offset.y);
                self.clamp_current_pan();
                self.view_locked = true;
                Task::none()
            }
            Message::CanvasZoom { scale, pan_offset } => {
                self.pan_offset = (pan_offset.x, pan_offset.y);
                self.set_scale(scale);
                self.view_locked = true;
                Task::none()
            }
            Message::RotateCW => {
                self.rotation = (self.rotation + 90) % 360;
                self.invalidate_cache();
                if !self.view_locked {
                    self.fit_view();
                } else {
                    self.clamp_current_pan();
                }
                Task::none()
            }
            Message::RotateCCW => {
                self.rotation = (self.rotation + 270) % 360;
                self.invalidate_cache();
                if !self.view_locked {
                    self.fit_view();
                } else {
                    self.clamp_current_pan();
                }
                Task::none()
            }
            Message::FlipHorizontal => {
                self.flip_h = !self.flip_h;
                self.invalidate_cache();
                self.clamp_current_pan();
                Task::none()
            }
            Message::FlipVertical => {
                self.flip_v = !self.flip_v;
                self.invalidate_cache();
                self.clamp_current_pan();
                Task::none()
            }
            Message::ResetTransform => {
                self.scale = 1.0;
                self.zoom_percent = 100;
                self.pan_offset = (0.0, 0.0);
                self.rotation = 0;
                self.flip_h = false;
                self.flip_v = false;
                self.invalidate_cache();
                Task::none()
            }
            Message::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;
                window::maximize(self.window_id, self.fullscreen)
            }
            Message::ToggleSlideshow => {
                self.slideshow_active = !self.slideshow_active;
                Task::none()
            }
            Message::AdjustSlideshowDelay(delta) => {
                if self.slideshow_interval as i64 + delta >= 1 {
                    self.slideshow_interval = (self.slideshow_interval as i64 + delta) as u64;
                }
                Task::none()
            }
            Message::SlideshowTick => {
                if self.slideshow_active && !self.paths.is_empty() {
                    self.current = (self.current + 1) % self.paths.len();
                    self.preload_adjacent();
                    if !self.view_locked {
                        self.fit_view();
                    }
                }
                Task::none()
            }
            Message::Close => iced::exit(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use iced::window;

    use super::{App, Message};

    fn app_with_image(image_size: (u32, u32)) -> App {
        App {
            window_id: window::Id::unique(),
            paths: vec!["missing-image".to_owned()],
            current: 0,
            image_cache: HashMap::new(),
            fullscreen: false,
            slideshow_active: false,
            slideshow_interval: 5,
            scale: 1.0,
            min_scale: 0.1,
            max_scale: 100.0,
            scale_step: 0.04,
            zoom_percent: 100,
            pan_offset: (0.0, 0.0),
            view_locked: false,
            rotation: 0,
            flip_h: false,
            flip_v: false,
            window_size: (800, 600),
            current_image_size: image_size,
        }
    }

    #[test]
    fn empty_image_set_stays_open_without_panicking() {
        let mut app = app_with_image((0, 0));
        app.paths.clear();

        let _ = app.update(Message::Next);
        let _ = app.view();
    }

    #[test]
    fn fit_view_allows_scales_below_the_interactive_floor() {
        let mut app = app_with_image((20_000, 15_000));

        app.fit_view();

        assert!((app.scale - 0.0376).abs() < f32::EPSILON);
        assert!(app.scale < app.min_scale);
    }

    #[test]
    fn keyboard_pan_keeps_part_of_the_image_visible() {
        let mut app = app_with_image((400, 300));

        let _ = app.update(Message::Pan(10_000, -10_000));

        assert_eq!(app.pan_offset, (600.0, -432.0));
    }
}
