mod app;
mod window;

use crate::app::App;

fn main() {
    simple_logger::SimpleLogger::new().without_timestamps().init().unwrap();

    let event_loop = winit::event_loop::EventLoop::new();

    let app = App::new(&event_loop);

    App::run(app, event_loop);
}
