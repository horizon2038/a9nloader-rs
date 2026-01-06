use crate::screen;

pub struct Bmp<'a> {
    width: usize,
    height: usize,
    pixel_width: usize,
    pixel_raw: &'a [u8],
}

impl Bmp<'_> {
    pub fn new(raw_bmp: &[u8]) -> Option<Bmp> {
        if raw_bmp.len() < 54 {
            return None; // Not enough data for BMP header
        }

        let dimensions = get_bmp_dimensions(raw_bmp)?;
        let header = &raw_bmp[0..54];
        let pixel_raw = &raw_bmp[54..];

        let pixel_width = match header[28] {
            24 => {
                // 24-bit bmp
                3
            }
            32 => {
                // 32-bit bmp
                4
            }
            _ => {
                return None; // Unsupported bit depth
            }
        };

        Some(Bmp {
            width: dimensions.0,
            height: dimensions.1,
            pixel_width,
            pixel_raw,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixel_raw(&self) -> &[u8] {
        self.pixel_raw
    }

    pub fn pixel_width(&self) -> usize {
        self.pixel_width
    }

    // return pixel with iter
    pub fn pixel_iter(&self) -> impl Iterator<Item = (usize, usize, screen::Color)> + '_ {
        self.pixel_raw
            .chunks(self.pixel_width)
            .enumerate()
            .flat_map(move |(i, chunk)| {
                if chunk.len() < self.pixel_width {
                    return None; // Skip incomplete pixels
                }
                let x = i % self.width;
                let y = i / self.width;
                let blue = chunk[0];
                let green = chunk[1];
                let red = chunk[2];
                let alpha = if self.pixel_width == 4 {
                    chunk[3]
                } else {
                    0xff // Default alpha for 24-bit BMP
                };
                Some((x, y, screen::Color {
                    red,
                    green,
                    blue,
                    alpha,
                }))
            })
    }
}

pub fn draw_bmp(splash: &[u8], start_x: usize, start_y: usize) {
    // let mut screen = screen::VgaScreen::new();
    // screen.clear();
    let screen = screen::current_screen();

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

pub fn draw_bmp_to_screen<Screen>(
    bmp: &Bmp,
    screen: &mut Screen,
    start_x: usize,
    start_y: usize,
    scale: f32,
) where
    Screen: screen::Screen,
{
    for (x, y, color) in bmp.pixel_iter() {
        let scaled_x = (x as f32 * scale) as usize + start_x;
        let scaled_y = (y as f32 * scale) as usize + start_y;

        if scaled_x < screen.width() && scaled_y < screen.height() {
            screen.draw_pixel(scaled_x, scaled_y, color);
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
