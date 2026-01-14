use iced::keyboard;

use crate::Message;

pub fn handle_key_event(
    key: &keyboard::Key,
    _ctrl: bool,
    _shift: bool,
    empty_mods: bool,
) -> Option<Message> {
    if !empty_mods {
        return None;
    }

    match key {
        keyboard::Key::Character(c) => match c.as_str() {
            "q" => Some(Message::Close),
            "f" => Some(Message::ToggleFullscreen),
            "+" | "=" => Some(Message::ZoomIn),
            "-" => Some(Message::ZoomOut),
            "1" => Some(Message::ActualSize),
            "0" => Some(Message::FitWindow),
            "r" => Some(Message::Reset),
            " " => Some(Message::ToggleSlideshow),
            "t" => Some(Message::AdjustSlideshowDelay(1)),
            "T" => Some(Message::AdjustSlideshowDelay(-1)),
            "n" => Some(Message::Next),
            "p" => Some(Message::Prev),
            "g" => Some(Message::GoToFirst),
            "G" => Some(Message::GoToLast),
            "i" => Some(Message::ZoomIn),
            "o" => Some(Message::ZoomOut),
            "h" => Some(Message::Prev),
            "l" => Some(Message::Next),
            _ => None,
        },
        keyboard::Key::Named(n) => match n {
            keyboard::key::Named::ArrowRight => Some(Message::Next),
            keyboard::key::Named::ArrowLeft => Some(Message::Prev),
            keyboard::key::Named::ArrowUp => Some(Message::ZoomIn),
            keyboard::key::Named::ArrowDown => Some(Message::ZoomOut),
            keyboard::key::Named::Escape => Some(Message::Close),
            _ => None,
        },
        _ => None,
    }
}
