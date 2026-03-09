use crate::loader::InitImageInfo;
use crate::loader::MemoryInfo;

pub const ARCH_INFO_MAX: usize = 128;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    pub memory_info: MemoryInfo,
    pub init_image_info: InitImageInfo,
    pub arch_info: [usize; 128],
}

impl BootInfo {
    pub fn new(
        memory_info: MemoryInfo,
        init_image_info: InitImageInfo,
        arch_info: [usize; ARCH_INFO_MAX],
    ) -> Self {
        BootInfo {
            memory_info,
            init_image_info,
            arch_info,
        }
    }
}

pub static mut BOOT_INFO: BootInfo = BootInfo {
    memory_info: MemoryInfo {
        memory_map: core::ptr::null_mut(),
        memory_map_count: 0,
        memory_size: 0,
    },
    init_image_info: InitImageInfo {
        loaded_address: 0,
        init_image_pages: 0,
        entry_point_virtual_address: 0,
        init_info_virtual_address: 0,
        init_ipc_buffer_virtual_address: 0,
    },
    arch_info: [0; ARCH_INFO_MAX],
};
