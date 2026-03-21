use formosaic::GameEngine;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default to info level; override with RUST_LOG env var (e.g. RUST_LOG=debug).
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();
    log::info!("Starting Formosaic (desktop)...");
    let event_loop = EventLoop::new()?;
    let mut engine = GameEngine::new();
    event_loop.run_app(&mut engine)?;
    Ok(())
}
