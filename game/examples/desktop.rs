use formosaic::{Formosaic, GameEngine};
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();
    let event_loop = EventLoop::new()?;
    let mut engine = GameEngine::new(Formosaic::new());
    event_loop.run_app(&mut engine)?;
    Ok(())
}
