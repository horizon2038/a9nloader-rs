use crate::screen;

use uefi::boot;
use uefi::proto::console::gop::BltPixel;
use uefi::proto::console::gop::PixelFormat;

use crate::loader;

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
        let mut gop =
            boot::open_protocol_exclusive::<uefi::proto::console::gop::GraphicsOutput>(gop_handle)
                .unwrap();

        let current_mode = gop.current_mode_info();
        let (width, height) = current_mode.resolution();
        let mut back_buffer = vec::Vec::new();
        back_buffer.resize(width * height, BltPixel::new(0, 0, 0));

        // configure the framebuffer information to boot_info for the kernel->user
        let (r, g, b) = match current_mode.pixel_format() {
            PixelFormat::Rgb => (
                loader::ColorField {
                    position: 0,
                    size: 8,
                },
                loader::ColorField {
                    position: 8,
                    size: 8,
                },
                loader::ColorField {
                    position: 16,
                    size: 8,
                },
            ),
            PixelFormat::Bgr => (
                loader::ColorField {
                    position: 16,
                    size: 8,
                },
                loader::ColorField {
                    position: 8,
                    size: 8,
                },
                loader::ColorField {
                    position: 0,
                    size: 8,
                },
            ),
            PixelFormat::Bitmask => {
                let bit_mask = current_mode
                    .pixel_bitmask()
                    .expect("Failed to get pixel bitmask");
                let to_color_field = |mask: u32| -> loader::ColorField {
                    let position = mask.trailing_zeros() as u8;
                    let size = (mask.count_ones()) as u8;
                    loader::ColorField { position, size }
                };
                let r = to_color_field(bit_mask.red);
                let g = to_color_field(bit_mask.green);
                let b = to_color_field(bit_mask.blue);

                (r, g, b)
            }
            PixelFormat::BltOnly => {
                panic!("BltOnly pixel format is not supported for direct framebuffer access");
            }
        };

        let frame_buffer_info = loader::FramebufferInfo {
            address: gop.frame_buffer().as_mut_ptr() as usize,
            width: width as u32,
            height: height as u32,
            stride: current_mode.stride() as u32,
            bits_per_pixel: 32, // UEFI GOP typically uses 32 bits per pixel
            red: r,
            green: g,
            blue: b,
            alpha: loader::ColorField {
                position: 24,
                size: 8,
            },
        };

        let serialized_info = frame_buffer_info.serialize();

        unsafe {
            loader::BOOT_INFO.arch_info[1..14].copy_from_slice(&serialized_info);
        }

        VgaScreen {
            screen_width: width,
            screen_height: height,
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

    pub fn present_rect(
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
        if x >= self.screen_width || y >= self.screen_height {
            return; // Out of bounds, do nothing
        }

        let index = self.index(x, y);
        self.back_buffer[index] = Self::to_blt(color);
        // let _ = self.present_pixel(x, y);
    }

    fn flush(&mut self, x: usize, y: usize) {
        let _ = self.present_pixel(x, y);
    }

    fn flush_rect(&mut self, x: usize, y: usize, width: usize, height: usize) {
        let _ = self.present_rect(x, y, width, height);
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

            <Self as screen::Screen>::draw_pixel(
                self,
                x,
                y,
                screen::Color {
                    red: color.r(),
                    green: color.g(),
                    blue: color.b(),
                    alpha: 0xff, // Assuming full opacity
                },
            );
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
pub fn current_screen() -> &'static mut VgaScreen {
    #[allow(static_mut_refs)]
    unsafe {
        SCREEN.as_mut().unwrap()
    }
}
