fn main() -> iced::Result {
    iced::application(
        lychee_core::App::new,
        lychee_core::App::update,
        lychee_core::App::view,
    )
    .subscription(lychee_core::App::subscription)
    .run()
}
