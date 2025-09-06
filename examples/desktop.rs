use formosaic::game_engine::GameEngine;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("Starting Formosaic (winit/glutin) desktop...");
    let event_loop = EventLoop::new()?;
    let mut engine = GameEngine::new();
    event_loop.run_app(&mut engine)?;
    Ok(())
}
