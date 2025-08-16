use crate::screen;

use core::slice;
use uefi::boot;
use uefi::proto::console::gop::BltPixel;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Point, Size};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;

extern crate alloc;
use alloc::vec;

pub struct VgaScreen {
    screen_width: usize,
    screen_height: usize,
    gop: uefi::boot::ScopedProtocol<uefi::proto::console::gop::GraphicsOutput>,

    back_buffer: vec::Vec<BltPixel>,
}

impl VgaScreen {
    pub fn new() -> Self {
        let gop_handle =
            boot::get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>().unwrap();
        let gop =
            boot::open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(gop_handle)
                .unwrap();

        let (width, height) = gop.current_mode_info().resolution();
        let mut back_buffer = vec::Vec::new();
        back_buffer.resize(width * height, BltPixel::new(0, 0, 0));

        VgaScreen {
            screen_width: width as usize,
            screen_height: height as usize,
            gop,
            back_buffer,
        }
    }

    #[inline]
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.screen_width + x
    }

    #[inline]
    fn to_blt(color: screen::Color) -> BltPixel {
        BltPixel::new(color.red, color.green, color.blue)
    }

    #[inline]
    fn from_blt(pixel: BltPixel) -> screen::Color {
        screen::Color {
            red: pixel.red,
            green: pixel.green,
            blue: pixel.blue,
            alpha: 0xff, // Assuming full opacity
        }
    }

    pub fn present(&mut self) -> Result<(), uefi::Error> {
        self.gop
            .blt(uefi::proto::console::gop::BltOp::BufferToVideo {
                buffer: &self.back_buffer,
                src: uefi::proto::console::gop::BltRegion::Full,
                dest: (0, 0),
                dims: (self.screen_width, self.screen_height),
            })
    }

    pub fn present_pixel(&mut self, x: usize, y: usize) -> Result<(), uefi::Error> {
        self.gop
            .blt(uefi::proto::console::gop::BltOp::BufferToVideo {
                buffer: &self.back_buffer,
                src: uefi::proto::console::gop::BltRegion::SubRectangle {
                    coords: (x, y),
                    px_stride: self.screen_width,
                },
                dest: (x, y),
                dims: (1, 1),
            })
    }

    pub fn present_region(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<(), uefi::Error> {
        self.gop
            .blt(uefi::proto::console::gop::BltOp::BufferToVideo {
                buffer: &self.back_buffer,
                src: uefi::proto::console::gop::BltRegion::SubRectangle {
                    coords: (x, y),
                    px_stride: self.screen_width,
                },
                dest: (x, y),
                dims: (width, height),
            })
    }
}

// TODO: implement double buffering
impl screen::Screen for VgaScreen {
    fn pixel_at(&mut self, x: usize, y: usize) -> screen::Color {
        let pixel = self.back_buffer[self.index(x, y)];
        Self::from_blt(pixel)
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
        let length = self.screen_width * self.screen_height * core::mem::size_of::<BltPixel>();
        let pointer = self.back_buffer.as_mut_ptr() as *mut u8;
        unsafe { core::slice::from_raw_parts_mut(pointer, length) }
    }

    fn clear(&mut self) {
        let black = BltPixel::new(0, 0, 0);
        let _ = self.gop.blt(uefi::proto::console::gop::BltOp::VideoFill {
            color: black,
            dest: (0, 0),
            dims: (self.screen_width, self.screen_height),
        });

        for pixel in &mut self.back_buffer {
            *pixel = black;
        }

        let _ = uefi::system::with_stdout(|stdout| {
            let _ = uefi::proto::console::text::Output::clear(stdout);
            uefi::proto::console::text::Output::reset(stdout, true)
        });
    }

    fn draw_pixel(&mut self, x: usize, y: usize, color: screen::Color) {
        let index = self.index(x, y);
        self.back_buffer[index] = Self::to_blt(color);
        // let _ = self.present_pixel(x, y);
    }

    fn flush(&mut self, x: usize, y: usize) {
        let _ = self.present_pixel(x, y);
    }

    fn flush_all(&mut self) {
        let _ = self.present();
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

            <Self as screen::Screen>::draw_pixel(self, x, y, screen::Color {
                red: color.r(),
                green: color.g(),
                blue: color.b(),
                alpha: 0xff, // Assuming full opacity
            });
        }
        // <Self as screen::Screen>::flush_all(self);

        Ok(())
    }
}

pub fn init_screen() {
    unsafe {
        SCREEN = Some(VgaScreen::new());
    }
}

pub static mut SCREEN: Option<VgaScreen> = None;
