//! Application host — the `Application` trait and the `GameEngine` host.

pub mod application;

#[cfg(feature = "windowed")]
pub mod game_engine;
