#![no_main]
#![no_std]
#![allow(dead_code)]

mod screen;
use screen::Screen;
mod gui;
mod loader;
mod print;
mod util;

use uefi::prelude::*;

#[entry]
fn main() -> Status {
    uefi_init();
    gui_init();

    log_a9nloader_info();

    loader::run().unwrap_or_else(|e| {
        error!("Failed to run loader: {}", e);
    });

    loop {}

    Status::SUCCESS
}

fn uefi_init() {
    let _ = uefi::helpers::init();
    let _ = uefi::system::with_stdout(|stdout| {
        let _ = uefi::proto::console::text::Output::clear(stdout);
        let _ = uefi::proto::console::text::Output::reset(stdout, true);
    });

    screen::init_screen();
}

const A9NLOADER_LOGO: &str = r#"
    _   ___  _   _ _                    _           
   / \ / _ \| \ | | |    ___   __ _  __| | ___ _ __ 
  / _ \ (_) |  \| | |   / _ \ / _` |/ _` |/ _ \ '__|
 / ___ \__, | |\  | |__| (_) | (_| | (_| |  __/ |   
/_/   \_\/_/|_| \_|_____\___/ \__,_|\__,_|\___|_|   
"#;

const A9N_SPLASH_BMP: &[u8] = include_bytes!("../resources/a9n-project.bmp");
const A9N_LOADER_SPLASH_BMP: &[u8] = include_bytes!("../resources/a9n-loader.bmp");

fn gui_init() {
    gui::draw_bmp(A9N_LOADER_SPLASH_BMP, 0, 0);

    let screen = screen::current_screen();
    let width = screen.width();
    let height = screen.height();
    // let header_dimensions = gui::get_bmp_dimensions(A9N_SPLASH_BMP).unwrap_or((0, 0));

    for y in 80..height {
        for x in 0..width {
            screen.draw_pixel(x, y, screen::Color {
                red: 0x14,
                green: 0x14,
                blue: 0x14,
                alpha: 0xff,
            });
        }
    }
}

fn log_a9nloader_info() {
    println!("{}", A9NLOADER_LOGO);
    info!(
        "A9NLoader-rs v{}, written by {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS"),
    );
}
