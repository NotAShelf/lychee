use std::collections::HashMap;

use iced::{
    ContentFit, Element, Subscription, Task, keyboard,
    time::{self, milliseconds},
    widget::image::viewer,
    window,
};
use image::GenericImageView;
use lychee_cli::{Args, Clap};
use lychee_img::collect_paths;

use crate::keybindings::handle_key_event;

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
    Reset,
    ToggleFullscreen,
    ToggleSlideshow,
    SlideshowTick,
    AdjustSlideshowDelay(i64),
    Close,
}

pub struct App {
    window_id: window::Id,
    paths: Vec<String>,
    current: usize,
    image_cache: HashMap<usize, iced::widget::image::Handle>,
    fit_to_window: bool,
    fullscreen: bool,
    slideshow_active: bool,
    slideshow_interval: u64,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let args = Args::parse();
        let paths = collect_paths(args.paths);
        let window_id = window::Id::unique();

        let app = Self {
            window_id,
            paths,
            current: 0,
            image_cache: HashMap::new(),
            fit_to_window: true,
            fullscreen: args.fullscreen,
            slideshow_active: false,
            slideshow_interval: args.slideshow,
        };

        let task = if args.fullscreen {
            window::maximize(window_id, true)
        } else {
            Task::none()
        };

        (app, task)
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.image_cache.get(&self.current) {
            Some(handle) => {
                let v = viewer(handle.clone())
                    .content_fit(if self.fit_to_window {
                        ContentFit::Contain
                    } else {
                        ContentFit::Cover
                    })
                    .padding(20);
                v.into()
            }

            None => iced::widget::text("No image loaded").into(),
        }
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

        Subscription::batch(vec![hotkey, close, slideshow])
    }

    fn preload_adjacent(&mut self) {
        for offset in [0isize, -1, 1] {
            let idx = (self.current as isize + offset) as usize;
            if idx < self.paths.len()
                && !self.image_cache.contains_key(&idx)
                && let Some(handle) = self.load_image(&self.paths[idx])
            {
                self.image_cache.insert(idx, handle);
            }
        }
    }

    fn load_image(&self, path: &str) -> Option<iced::widget::image::Handle> {
        let img = ::image::open(path).ok()?;
        let (width, height) = img.dimensions();
        let raw = img.to_rgba8().into_raw();
        Some(iced::widget::image::Handle::from_rgba(width, height, raw))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
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
            Message::ZoomIn
            | Message::ZoomOut
            | Message::ActualSize
            | Message::FitWindow
            | Message::Reset => Task::none(),
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
