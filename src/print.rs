use crate::screen;
use core::fmt::Write;
use uefi;

extern crate alloc;
use alloc::vec;

use embedded_graphics;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::*},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::Rectangle,
    text::renderer::CharacterStyle,
};
use embedded_text::{
    TextBox,
    alignment::HorizontalAlignment,
    plugin::ansi::Ansi,
    style::{HeightMode, TextBoxStyle, TextBoxStyleBuilder},
};

struct VirtualConsole<'a> {
    textbox_style: TextBoxStyle,
    character_style: MonoTextStyle<'a, Rgb888>,
    cursor: Point,
    line_buffer: vec::Vec<u8>,
}

const CONSOLE_WIDTH_OFFSET: i32 = 10;
const CONSOLE_HEIGHT_OFFSET: i32 = 80;

impl core::fmt::Write for VirtualConsole<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        #[allow(static_mut_refs)]
        unsafe {
            screen::SCREEN.as_mut().and_then(|screen| {
                let line_height = self.character_style.font.character_size.height as i32;
                let screen_height = screen.bounding_box().size.height as i32;

                // split lines
                for line in s.lines() {
                    // scroll
                    if self.cursor.y + line_height > screen_height {
                        screen.clear(Rgb888::BLACK).ok(); // エラーは無視
                        // clear lines

                        // reset cursor position
                        self.cursor.y = CONSOLE_HEIGHT_OFFSET;
                    }

                    if !line.is_empty() {
                        let bounds = Rectangle::new(
                            self.cursor,
                            // Size::new(screen.size().width - self.cursor.x as u32, 0),
                            Size::new(
                                screen.size().width,
                                self.character_style.font.character_size.height as u32,
                            ),
                        );

                        let mut textbox = TextBox::with_textbox_style(
                            line,
                            bounds,
                            self.character_style,
                            self.textbox_style,
                        )
                        .add_plugin(Ansi::new());

                        let _ = textbox.draw(screen);
                    }

                    self.cursor.y += line_height;
                }
                let _ = screen::Screen::flush_all(screen);
                Some(())
            });
        }

        Ok(())
    }
}

static mut VIRTUAL_CONSOLE: Option<VirtualConsole<'static>> = None;

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    unsafe {
        #[allow(static_mut_refs)]
        if VIRTUAL_CONSOLE.is_none() {
            let character_style = MonoTextStyle::new(&FONT_6X12, Rgb888::WHITE);
            let line_height = character_style.font.character_size.height as i32;
            let textbox_style = TextBoxStyleBuilder::new()
                .height_mode(HeightMode::Exact(
                    embedded_text::style::VerticalOverdraw::Hidden,
                ))
                .line_height(embedded_graphics::text::LineHeight::Pixels(
                    line_height as u32,
                ))
                .trailing_spaces(false)
                .paragraph_spacing(0)
                .build();

            let mut line_buffer = vec::Vec::<u8>::new();
            line_buffer.resize(512, 0);

            VIRTUAL_CONSOLE = Some(VirtualConsole {
                textbox_style,
                character_style,
                cursor: Point::new(CONSOLE_WIDTH_OFFSET, CONSOLE_HEIGHT_OFFSET),
                line_buffer,
            });
        }

        #[allow(static_mut_refs)]
        let _ = VIRTUAL_CONSOLE.as_mut().and_then(|virtual_console| {
            let _ = virtual_console.write_fmt(args);

            Some(())
        });

        uefi::system::with_stdout(|stdout| {
            stdout.write_fmt(args).expect("Failed to write to stdout");
        });
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        extern crate alloc;
        let __line = alloc::format!($($arg)*);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::print::_print(core::format_args!("\n"));
    }};
    ($fmt:literal $(, $($arg:tt)+)?) => {{
        extern crate alloc;
        let __line = alloc::format!(concat!($fmt, "\n") $(, $($arg)+)?);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
    ($($arg:tt)*) => {{
        let __payload = alloc::format!($($arg)*);
        let __line = alloc::format!("{}\n", __payload);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
}

#[macro_export]
macro_rules! info {
    ($fmt:literal $(, $($arg:tt)+)?) => {{
        extern crate alloc;
        let __line = alloc::format!(concat!("[\x1b[32m INFO\x1b[37m] ", $fmt, "\n") $(, $($arg)+)?);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
    ($($arg:tt)*) => {{
        let __payload = alloc::format!($($arg)*);
        let __line = alloc::format!("[\x1b[32m INFO\x1b[37m] {}\n", __payload);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
}

#[macro_export]
macro_rules! warn {
    ($fmt:literal $(, $($arg:tt)+)?) => {{
        extern crate alloc;
        let __line = alloc::format!(concat!("[\x1b[33m WARN\x1b[37m] ", $fmt, "\n") $(, $($arg)+)?);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
    ($($arg:tt)*) => {{
        let __payload = alloc::format!($($arg)*);
        let __line = alloc::format!("[\x1b[33m WARN\x1b[37m] {}\n", __payload);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
}

#[macro_export]
macro_rules! error {
    ($fmt:literal $(, $($arg:tt)+)?) => {{
        extern crate alloc;
        let __line = alloc::format!(concat!("[\x1b[31mERROR\x1b[37m] ", $fmt, "\n") $(, $($arg)+)?);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
    ($($arg:tt)*) => {{
        let __payload = alloc::format!($($arg)*);
        let __line = alloc::format!("[\x1b[31mERROR\x1b[37m] {}\n", __payload);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
}

#[macro_export]
macro_rules! debug {
    ($fmt:literal $(, $($arg:tt)+)?) => {{
        extern crate alloc;
        let __line = alloc::format!(concat!("[\x1b[34mDEBUG\x1b[37m] ", $fmt, "\n") $(, $($arg)+)?);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
    ($($arg:tt)*) => {{
        let __payload = alloc::format!($($arg)*);
        let __line = alloc::format!("[\x1b[34mDEBUG\x1b[37m] {}\n", __payload);
        $crate::print::_print(core::format_args!("{}", __line.as_str()));
    }};
}
