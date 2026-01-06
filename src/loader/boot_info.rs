use crate::loader::InitImageInfo;
use crate::loader::MemoryInfo;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    pub memory_info: MemoryInfo,
    pub init_image_info: InitImageInfo,
    pub arch_info: [usize; 8],
}

impl BootInfo {
    pub fn new(
        memory_info: MemoryInfo,
        init_image_info: InitImageInfo,
        arch_info: [usize; 8],
    ) -> Self {
        BootInfo {
            memory_info,
            init_image_info,
            arch_info,
        }
    }
}
