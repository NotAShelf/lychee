use std::collections::HashMap;

use iced::event;
use iced::Point;
use iced::{
    keyboard,
    time::{self, milliseconds},
    widget::{column, row},
    window, Element, Length, Subscription, Task,
};
use image::GenericImageView;
use lychee_cli::{Args, Clap};
use lychee_img::collect_paths;
use lychee_widgets::ImageCanvas;

use crate::keybindings::handle_key_event;

type ImageCacheKey = (usize, u16, bool, bool);
type ImageCacheValue = (iced::widget::image::Handle, u32, u32);

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
    CanvasWheelZoom {
        factor: f32,
        cursor: Point,
        viewport_center: Point,
    },
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
            max_scale: 10.0,
            scale_step: 0.1,
            zoom_percent: 100,
            pan_offset: (0.0, 0.0),
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

        let task = if args.fullscreen {
            window::maximize(window_id, true)
        } else {
            Task::none()
        };

        (app, task)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let image_content: Element<'_, Message> =
            match self
                .image_cache
                .get(&(self.current, self.rotation, self.flip_h, self.flip_v))
            {
                Some((handle, width, height)) => {
                    // Create ImageCanvas with pan/zoom support
                    // zoom_step = 1 + scale_step (e.g., 1.0 + 0.1 = 1.1 for 10% steps)
                    let zoom_step = 1.0 + self.scale_step;
                    let canvas = ImageCanvas::new(
                        handle.clone(),
                        *width,
                        *height,
                        self.scale,
                        Point::new(self.pan_offset.0, self.pan_offset.1),
                        zoom_step,
                    );

                    canvas.into_element().map(|msg| match msg {
                        lychee_widgets::Message::Pan(dx, dy) => Message::Pan(dx as i32, dy as i32),
                        lychee_widgets::Message::Zoom {
                            factor,
                            cursor,
                            viewport_center,
                        } => Message::CanvasWheelZoom {
                            factor,
                            cursor,
                            viewport_center,
                        },
                        lychee_widgets::Message::ZoomIn => Message::ZoomIn,
                        lychee_widgets::Message::ZoomOut => Message::ZoomOut,
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
            let idx = (self.current as isize + offset) as usize;
            let cache_key = (idx, self.rotation, self.flip_h, self.flip_v);
            if idx < self.paths.len() && !self.image_cache.contains_key(&cache_key) {
                // Extract path to avoid borrow conflict
                let path = self.paths[idx].clone();
                if let Some(handle_data) = self.load_image(&path) {
                    self.image_cache.insert(cache_key, handle_data);
                }
            }
        }
    }

    fn load_image(&mut self, path: &str) -> Option<(iced::widget::image::Handle, u32, u32)> {
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

        // Track original image dimensions for fit-to-window calculation
        let (orig_width, orig_height) = img.dimensions();
        self.current_image_size = (orig_width, orig_height);

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
            let idx = (self.current as isize + offset) as usize;
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
                self.image_cache.insert(key, handle_data);
            }
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // Ensure current image is loaded; this handles edge cases where
        // cache miss occurred and preload hasn't triggered yet
        self.ensure_current_loaded();

        match message {
            Message::Next => {
                if self.current < self.paths.len() - 1 {
                    self.current += 1;
                    self.preload_adjacent();
                }
                Task::none()
            }
            Message::Prev => {
                if self.current > 0 {
                    self.current -= 1;
                    self.preload_adjacent();
                }
                Task::none()
            }
            Message::GoToFirst => {
                if !self.paths.is_empty() {
                    self.current = 0;
                    self.preload_adjacent();
                }
                Task::none()
            }
            Message::GoToLast => {
                if !self.paths.is_empty() {
                    self.current = self.paths.len() - 1;
                    self.preload_adjacent();
                }
                Task::none()
            }
            Message::ZoomIn => {
                // Zoom is visual only via canvas transform - no cache invalidation
                let new_percent =
                    (self.zoom_percent as f32 * (1.0 + self.scale_step)).round() as u32;
                self.zoom_percent = new_percent.min((self.max_scale * 100.0) as u32);
                self.scale = self.zoom_percent as f32 / 100.0;
                // No cache invalidation - image data unchanged
                Task::none()
            }
            Message::ZoomOut => {
                // Zoom is visual only via canvas transform - no cache invalidation
                let new_percent =
                    (self.zoom_percent as f32 * (1.0 / (1.0 + self.scale_step))).round() as u32;
                self.zoom_percent = new_percent.max((self.min_scale * 100.0) as u32);
                self.scale = self.zoom_percent as f32 / 100.0;
                // No cache invalidation - image data unchanged
                Task::none()
            }
            Message::ActualSize | Message::FitWindow | Message::ResetView => {
                let fit_zoom = if self.current_image_size.0 > 0 && self.current_image_size.1 > 0 {
                    // Calculate zoom to fit image within window, preserving aspect ratio
                    let window_ratio = (self.window_size.0 as f32) / (self.window_size.1 as f32);
                    let image_ratio =
                        (self.current_image_size.0 as f32) / (self.current_image_size.1 as f32);

                    if window_ratio > image_ratio {
                        // Window is wider than image, fit by height
                        ((self.window_size.1 as f32) / (self.current_image_size.1 as f32) * 100.0)
                            as u32
                    } else {
                        // Window is taller than image, fit by width
                        ((self.window_size.0 as f32) / (self.current_image_size.0 as f32) * 100.0)
                            as u32
                    }
                } else {
                    100 // default to 100% if no image dimensions available
                };

                self.zoom_percent = fit_zoom.min((self.max_scale * 100.0) as u32);
                self.scale = self.zoom_percent as f32 / 100.0;
                self.pan_offset = (0.0, 0.0);
                self.invalidate_cache();
                Task::none()
            }
            Message::WindowResized(width, height) => {
                self.window_size = (width, height);
                Task::none()
            }
            Message::Pan(dx, dy) => {
                // Update visual pan offset from canvas drag
                // Canvas sends raw screen pixel delta; keyboard sends 50 units per press
                // Use raw values directly for natural mouse feel
                self.pan_offset.0 += dx as f32;
                self.pan_offset.1 += dy as f32;
                Task::none()
            }
            Message::CanvasWheelZoom {
                factor,
                cursor,
                viewport_center,
            } => {
                // Handle wheel zoom from canvas - zoom is visual only. Zoom focuses
                // around cursor position
                let old_scale = self.scale;
                let new_scale = (self.scale * factor).clamp(self.min_scale, self.max_scale);

                // Use viewport center from canvas. NOT window center, because canvas may not fill window
                // and instead go piss in my cereal. Fuck. FUCK.
                if (new_scale - old_scale).abs() > f32::EPSILON {
                    // Calculate cursor offset from viewport center
                    let cursor_offset =
                        Point::new(cursor.x - viewport_center.x, cursor.y - viewport_center.y);

                    // Calculate pan adjustment for zoom-around-cursor
                    // Formula: pan_delta = cursor_offset * (1 - 1/factor) / old_scale
                    // This keeps the point under cursor stationary during zoom
                    let pan_delta = (
                        cursor_offset.x * (1.0 - 1.0 / factor) / old_scale,
                        cursor_offset.y * (1.0 - 1.0 / factor) / old_scale,
                    );

                    self.pan_offset.0 += pan_delta.0;
                    self.pan_offset.1 += pan_delta.1;

                    self.scale = new_scale;
                    self.zoom_percent = (new_scale * 100.0).round() as u32;
                }
                Task::none()
            }
            Message::RotateCW => {
                self.rotation = (self.rotation + 90) % 360;
                self.invalidate_cache();
                Task::none()
            }
            Message::RotateCCW => {
                self.rotation = (self.rotation + 270) % 360;
                self.invalidate_cache();
                Task::none()
            }
            Message::FlipHorizontal => {
                self.flip_h = !self.flip_h;
                self.invalidate_cache();
                Task::none()
            }
            Message::FlipVertical => {
                self.flip_v = !self.flip_v;
                self.invalidate_cache();
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
                }
                Task::none()
            }
            Message::Close => iced::exit(),
        }
    }
}
