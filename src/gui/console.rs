use crate::util::*;

pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

pub trait Console {
    fn write_str(&self, s: &str) -> BootResult<()>;
    fn clear(&self) -> BootResult<()>;
    fn cursor(&self) -> BootResult<Cursor>;
    fn set_cursor(&self, cursor: &Cursor) -> BootResult<()>;
}
