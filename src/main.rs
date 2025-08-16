#![no_main]
#![no_std]
#![allow(dead_code)]

mod screen;
use screen::Screen;

mod print;
use print::*;

use core::option::Option;

use embedded_graphics;
use uefi::prelude::*;
// use uefi::println;

#[entry]
fn main() -> Status {
    uefi_init();
    log_a9nloader_info();

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

const A9N_SPLASH_BMP: &[u8] = include_bytes!("../a9n-project.bmp");

fn log_a9nloader_info() {
    draw_bmp(A9N_SPLASH_BMP);

    println!("{}", A9NLOADER_LOGO);
    println!(
        "A9NLoader-rs v{}, written by {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );
    println!("Press any key to continue...");

    info!("A9NLoader initialized successfully.");
    warn!("This is a test warning message.");
    error!("This is a test error message.");
    debug!("This is a test debug message.");
    /*
    for _ in 0..100 {
        println!("test...");
    }
    */
}

fn draw_bmp(splash: &[u8]) {
    // let mut screen = screen::VgaScreen::new();
    #[allow(static_mut_refs)]
    let mut screen = unsafe { screen::SCREEN.as_mut().unwrap() };
    // screen.clear();

    let bmp_dimensions = get_bmp_dimensions(splash).unwrap_or((0, 0));

    let scale_x = screen.width() as f32 / bmp_dimensions.0 as f32;
    let scale_y = screen.height() as f32 / bmp_dimensions.1 as f32;
    let scale = if scale_x < scale_y { scale_x } else { scale_y };

    let scaled_width = (bmp_dimensions.0 as f32 * scale) as usize;
    let scaled_height = (bmp_dimensions.1 as f32 * scale) as usize;

    let start_x = (screen.width() - scaled_width) / 2;
    let start_y = (screen.height() - scaled_height) / 2;

    let pixel_data = &splash[54..];

    for y in 0..scaled_height {
        for x in 0..scaled_width {
            let src_x = (x as f32 / scale) as usize;
            let src_y = (y as f32 / scale) as usize;

            let offset = (src_y * bmp_dimensions.0 + src_x) * 3;
            if offset + 2 >= pixel_data.len() {
                continue; // Prevent out-of-bounds access
            }

            let blue = pixel_data[offset];
            let green = pixel_data[offset + 1];
            let red = pixel_data[offset + 2];

            screen.draw_pixel(start_x + x, start_y + y, screen::Color {
                red,
                green,
                blue,
                alpha: 0xff,
            });
        }
    }
}

fn get_bmp_dimensions(header: &[u8]) -> Option<(usize, usize)> {
    if header.len() < 18 {
        return None; // Not enough data for dimensions
    }

    // width: 18 +4
    let width_bytes = &header[18..22];
    // height: 22 +4
    let height_bytes = &header[22..26];

    let width = u32::from_le_bytes([
        width_bytes[0],
        width_bytes[1],
        width_bytes[2],
        width_bytes[3],
    ]) as usize;

    // height is stored as a signed integer in BMP files, but we treat it as unsigned here.
    let height_raw = i32::from_le_bytes([
        height_bytes[0],
        height_bytes[1],
        height_bytes[2],
        height_bytes[3],
    ]);
    let height = height_raw.abs() as usize;

    Some((width, height))
}
