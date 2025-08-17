extern crate alloc;
use alloc::vec;

use uefi::boot::{MemoryDescriptor, MemoryType};
use uefi::{boot, system};

use crate::util::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum MemoryMapType {
    Free,
    Device,
    Reserved,
}

#[repr(C)]
pub struct MemoryMapEntry {
    pub physical_address_start: usize,
    pub page_count: usize,
    pub memory_type: MemoryMapType,
}

#[repr(C)]
pub struct MemoryInfo {
    pub memory_size: usize,
    pub memory_map_count: u16,
    pub memory_map: *mut MemoryMapEntry,
}
