use env_logger;
use winit::event_loop::EventLoop;

use formosaic::shared;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("Starting Formosaic (winit/glutin) desktop...");
    let event_loop = EventLoop::new()?;
    shared::run_engine(event_loop);
    Ok(())
}
