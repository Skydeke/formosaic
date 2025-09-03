#![cfg(android_platform)]

use android_logger::Config;
use log::LevelFilter;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;

use formosaic::shared;

#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    std::env::set_var("RUST_BACKTRACE", "full");
    android_logger::init_once(Config::default().with_max_level(LevelFilter::Info));
    log::info!("Starting Formosaic (winit/glutin) on Android...");
    let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
    shared::run_engine(event_loop);
}
