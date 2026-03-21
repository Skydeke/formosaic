#![cfg(target_os = "android")]

use android_logger::Config;
use formosaic::{Formosaic, GameEngine};
use log::LevelFilter;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;

#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    std::env::set_var("RUST_BACKTRACE", "full");
    android_logger::init_once(Config::default().with_max_level(LevelFilter::Info));
    log::info!("Starting Formosaic (Android)...");
    let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
    let mut engine = GameEngine::new(Formosaic::new());
    if let Err(e) = event_loop.run_app(&mut engine) {
        log::error!("Event loop terminated with error: {:?}", e);
    }
}
