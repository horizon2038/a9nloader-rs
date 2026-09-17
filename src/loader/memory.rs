use uefi::boot::MemoryType;
use uefi::mem::memory_map::{MemoryMap, MemoryMapMut};

use crate::util::*;

mod map;
mod policy;
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
    let entries = buffer.entries().map(|entry| {
        let memory_type = policy::classify(entry.ty);
        if matches!(
            entry.ty,
            MemoryType::ACPI_RECLAIM | MemoryType::ACPI_NON_VOLATILE
        ) {
            crate::info!(
                "ACPI memory: type={:?} paddr={:#x} pages={:#x} -> {:?}",
                entry.ty,
                entry.phys_start,
                entry.page_count,
                memory_type
            );
        }
        MemoryMapEntry {
            physical_address_start: entry.phys_start as usize,
            page_count: entry.page_count as usize,
            memory_type,
        }
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
