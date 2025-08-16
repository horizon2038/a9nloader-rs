use crate::screen;

use core::slice;
use embedded_graphics::pixelcolor::Rgb888;
use uefi::boot;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point, Size};
use embedded_graphics::prelude::*;

pub struct VgaScreen {
    screen_width: usize,
    screen_height: usize,
    gop: uefi::boot::ScopedProtocol<uefi::proto::console::gop::GraphicsOutput>,
}

impl VgaScreen {
    pub fn new() -> Self {
        let gop_handle =
            boot::get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>().unwrap();
        let gop =
            boot::open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(gop_handle)
                .unwrap();

        let (width, height) = gop.current_mode_info().resolution();
        VgaScreen {
            screen_width: width as usize,
            screen_height: height as usize,
            gop,
        }
    }
}

// TODO: implement double buffering
impl screen::Screen for VgaScreen {
    fn pixel_at(&mut self, x: usize, y: usize) -> screen::Color {
        // This is a stub implementation; actual pixel reading would require more work.
        let offset = ((y * self.width()) + x) * 4;
        screen::Color {
            red: unsafe { self.gop.frame_buffer().read_byte(offset + 2) },
            green: unsafe { self.gop.frame_buffer().read_byte(offset + 1) },
            blue: unsafe { self.gop.frame_buffer().read_byte(offset + 0) },
            alpha: 0xff, // Assuming full opacity
        }
    }

    fn width(&self) -> usize {
        self.screen_width
    }

    fn height(&self) -> usize {
        self.screen_height
    }

    fn mode(&self) -> screen::Mode {
        let mode_info = self.gop.current_mode_info();
        match mode_info.pixel_format() {
            uefi::proto::console::gop::PixelFormat::Bgr => screen::Mode::BGRA,
            uefi::proto::console::gop::PixelFormat::Rgb => screen::Mode::RGBA,
            _ => screen::Mode::BGRA, // Default to BGRA if unknown
        }
    }

    fn buffer_size(&self) -> usize {
        self.width() * self.height() * 4
    }

    fn raw_buffer(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.gop.frame_buffer().as_mut_ptr(),
                self.buffer_size(),
            )
        }
    }

    fn clear(&mut self) {
        for y in 0..self.height() {
            for x in 0..self.width() {
                self.draw_pixel(x, y, screen::Color {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 0xff,
                });
            }
        }

        let _ =
            uefi::system::with_stdout(|stdout| uefi::proto::console::text::Output::clear(stdout));
    }

    fn draw_pixel(&mut self, x: usize, y: usize, color: screen::Color) {
        let offset = ((y * self.width()) + x) * 4;
        let _ = unsafe {
            match self.mode() {
                screen::Mode::BGRA => {
                    self.gop.frame_buffer().write_byte(offset + 0, color.blue);
                    self.gop.frame_buffer().write_byte(offset + 1, color.green);
                    self.gop.frame_buffer().write_byte(offset + 2, color.red);
                    self.gop.frame_buffer().write_byte(offset + 3, color.alpha);
                }
                screen::Mode::RGBA => {
                    self.gop.frame_buffer().write_byte(offset + 0, color.red);
                    self.gop.frame_buffer().write_byte(offset + 1, color.green);
                    self.gop.frame_buffer().write_byte(offset + 2, color.blue);
                    self.gop.frame_buffer().write_byte(offset + 3, color.alpha);
                }
            }
        };
    }
}

impl OriginDimensions for VgaScreen {
    fn size(&self) -> Size {
        Size::new(
            <Self as screen::Screen>::width(self) as u32,
            <Self as screen::Screen>::height(self) as u32,
        )
    }
}

pub enum ScreenError {
    OutOfBounds,
}

impl DrawTarget for VgaScreen {
    type Color = Rgb888;
    type Error = ScreenError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(
            Point {
                x: point_x,
                y: point_y,
            },
            color,
        ) in pixels.into_iter()
        {
            if (point_x >= <Self as screen::Screen>::width(self) as i32)
                && (point_y >= <Self as screen::Screen>::height(self) as i32)
            {
                return Err(Self::Error::OutOfBounds);
            }

            let x = point_x as usize;
            let y = point_y as usize;

            let offset = ((y * <Self as screen::Screen>::width(self)) + x) * 4;

            match <Self as screen::Screen>::mode(self) {
                screen::Mode::BGRA => {
                    let frame_buffer = <Self as screen::Screen>::raw_buffer(self);
                    frame_buffer[offset] = color.b();
                    frame_buffer[offset + 1] = color.g();
                    frame_buffer[offset + 2] = color.r();
                    frame_buffer[offset + 3] = 0xff; // Assuming full opacity
                }
                screen::Mode::RGBA => {
                    let frame_buffer = <Self as screen::Screen>::raw_buffer(self);
                    frame_buffer[offset] = color.r();
                    frame_buffer[offset + 1] = color.g();
                    frame_buffer[offset + 2] = color.b();
                    frame_buffer[offset + 3] = 0xff; // Assuming full opacity
                }
            }
        }

        Ok(())
    }
}

pub fn init_screen() {
    unsafe {
        SCREEN = Some(VgaScreen::new());
    }
}

pub static mut SCREEN: Option<VgaScreen> = None;
