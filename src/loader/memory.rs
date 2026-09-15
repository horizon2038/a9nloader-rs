use uefi::boot::MemoryType;
use uefi::mem::memory_map::{MemoryMap, MemoryMapMut};

use crate::util::*;

mod map;
pub use map::{MemoryMapEntry, MemoryMapType};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryInfo {
    pub memory_size: usize,
    pub memory_map_count: u16,
    pub memory_map: *mut MemoryMapEntry,
}

static mut MEMORY_MAP_BUFFER: [MemoryMapEntry; 256] = [MemoryMapEntry::EMPTY; 256];

pub fn make_memory_info() -> BootResult<MemoryInfo> {
    let mut buffer = uefi::boot::memory_map(uefi::mem::memory_map::MemoryType::LOADER_DATA)?;
    // Firmware enumeration order is not necessarily physical address order.
    // Sort in place before deriving holes; no extra firmware allocation is needed.
    buffer.sort();
    let entries = buffer.entries().map(|entry| MemoryMapEntry {
        physical_address_start: entry.phys_start as usize,
        page_count: entry.page_count as usize,
        memory_type: match entry.ty {
            MemoryType::CONVENTIONAL | MemoryType::PERSISTENT_MEMORY => MemoryMapType::Free,
            MemoryType::RESERVED
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::RUNTIME_SERVICES_CODE
            | MemoryType::RUNTIME_SERVICES_DATA
            | MemoryType::UNUSABLE
            | MemoryType::ACPI_NON_VOLATILE
            | MemoryType::PAL_CODE => MemoryMapType::Reserved,
            // Firmware tables remain mappable, but are not normal allocator RAM.
            // This also preserves the existing treatment of loader/MMIO/unknown types.
            _ => MemoryMapType::Device,
        },
    });

    // Boot is single-threaded and the static output must survive ExitBootServices.
    let memory_map = &raw mut MEMORY_MAP_BUFFER;
    let memory_map_count = map::build_memory_map(
        entries,
        unsafe { &mut *memory_map },
        1usize << 46,
    )
    .map_err(|error| {
        uefi_error(match error {
            map::MapError::Capacity => uefi::Status::BUFFER_TOO_SMALL,
            map::MapError::InvalidRange | map::MapError::Overlap => uefi::Status::INVALID_PARAMETER,
        })
    })?;

    Ok(MemoryInfo {
        memory_size: 0, // currently unused
        memory_map_count: memory_map_count as u16,
        memory_map: memory_map.cast(),
    })
}
