use crate::screen;

pub fn draw_bmp(splash: &[u8], start_x: usize, start_y: usize) {
    // let mut screen = screen::VgaScreen::new();
    #[allow(static_mut_refs)]
    let mut screen = unsafe { screen::SCREEN.as_mut().unwrap() };
    // screen.clear();

    let bmp_dimensions = get_bmp_dimensions(splash).unwrap_or((0, 0));

    use crate::screen::Screen;
    let scale_x = screen.width() as f32 / bmp_dimensions.0 as f32;
    let scale_y = screen.height() as f32 / bmp_dimensions.1 as f32;
    let scale = if scale_x < scale_y { scale_x } else { scale_y };

    let scaled_width = (bmp_dimensions.0 as f32 * scale) as usize;
    let scaled_height = (bmp_dimensions.1 as f32 * scale) as usize;

    // let start_x = (screen.width() - scaled_width) / 2;
    // let start_y = (screen.height() - scaled_height) / 2;

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
    screen.flush_all();
}

pub fn get_bmp_dimensions(header: &[u8]) -> Option<(usize, usize)> {
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
