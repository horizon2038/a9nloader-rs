// interface for common screen

pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

pub enum Mode {
    BGRA,
    RGBA,
}

pub trait Screen {
    // basic properties
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn mode(&self) -> Mode;

    // frame buffer
    fn buffer_size(&self) -> usize;
    fn raw_buffer(&mut self) -> &mut [u8];

    // pixel operations
    fn clear(&mut self);
    fn draw_pixel(&mut self, x: usize, y: usize, color: Color);
    fn pixel_at(&mut self, x: usize, y: usize) -> Color;
}
