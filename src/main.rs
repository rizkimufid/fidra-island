//! Fidra Island — Windows Dynamic Island desktop media controller.
//!
//! Thin binary entrypoint delegating application lifecycle to [`app::run`].

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod config;
mod logging;
mod lyrics;
mod media;
mod platform;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    app::run()
}
