use crate::screen;
use core::fmt::Write;
use uefi;

use embedded_graphics;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_9X18},
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
}

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
                        // screen.clear(Rgb888::CSS_DARK_GRAY).ok(); // エラーは無視

                        // reset cursor position
                        self.cursor.y = 10;
                    }

                    if !line.is_empty() {
                        let bounds = Rectangle::new(
                            self.cursor,
                            Size::new(screen.size().width - self.cursor.x as u32, 0),
                        );

                        let textbox = TextBox::with_textbox_style(
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
            let character_style = MonoTextStyle::new(&FONT_9X18, Rgb888::WHITE);
            let textbox_style = TextBoxStyleBuilder::new()
                .height_mode(HeightMode::FitToText)
                .alignment(HorizontalAlignment::Left)
                .build();

            VIRTUAL_CONSOLE = Some(VirtualConsole {
                textbox_style,
                character_style,
                cursor: Point::new(10, 10),
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
    ($($arg:tt)*) => ($crate::print::_print(core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {
        $crate::print::_print(core::format_args!("{}{}", core::format_args!($($arg)*), "\n"))
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::println!("[\x1b[32m INFO\x1b[37m] {}", core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::println!("[\x1b[33m WARN\x1b[37m] {}", core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::println!("[\x1b[31mERROR\x1b[37m] {}", core::format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::println!("[\x1b[34mDEBUG\x1b[37m] {}", core::format_args!($($arg)*));
    };
}
