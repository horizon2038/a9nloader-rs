mod bmp;
pub use bmp::*;

mod console;

pub const A9N_SPLASH_BMP: &[u8] = include_bytes!("../resources/a9n-project.bmp");
pub const A9N_LOADER_SPLASH_BMP: &[u8] = include_bytes!("../resources/a9n-loader.bmp");
