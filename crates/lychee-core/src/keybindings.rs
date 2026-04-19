use iced::keyboard;

use crate::Message;

pub fn handle_key_event(
    key: &keyboard::Key,
    _ctrl: bool,
    shift: bool,
    empty_mods: bool,
) -> Option<Message> {
    // Handle shift-modified keys first
    if shift && empty_mods {
        if let keyboard::Key::Character(c) = key {
            match c.as_str() {
                "R" => return Some(Message::RotateCCW),
                "F" => return Some(Message::FlipVertical),
                "0" => return Some(Message::ResetTransform),
                _ => {}
            }
        }
        if let keyboard::Key::Named(n) = key {
            match n {
                keyboard::key::Named::ArrowRight => return Some(Message::Pan(50, 0)),
                keyboard::key::Named::ArrowLeft => return Some(Message::Pan(-50, 0)),
                keyboard::key::Named::ArrowUp => return Some(Message::Pan(0, -50)),
                keyboard::key::Named::ArrowDown => return Some(Message::Pan(0, 50)),
                _ => {}
            }
        }
    }

    // Block keys with other modifiers (ctrl, alt, meta)
    if !empty_mods {
        return None;
    }

    match key {
        keyboard::Key::Character(c) => match c.as_str() {
            "q" => Some(Message::Close),
            "f" => Some(Message::FlipHorizontal),
            "+" | "=" => Some(Message::ZoomIn),
            "-" => Some(Message::ZoomOut),
            "1" => Some(Message::ActualSize),
            "0" => Some(Message::FitWindow),
            "r" => Some(Message::RotateCW),
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
            keyboard::key::Named::F11 => Some(Message::ToggleFullscreen),
            _ => None,
        },
        _ => None,
    }
}
