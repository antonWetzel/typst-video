mod gui;
mod world;

use gui::Gui;
use iced::Application;

fn main() {
    video_rs::init().unwrap();
    Gui::run(iced::Settings::default()).unwrap();
}
