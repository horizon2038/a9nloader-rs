extern crate alloc;
use alloc::vec;

use uefi::boot::{MemoryDescriptor, MemoryType};
use uefi::mem::memory_map::MemoryMap;
use uefi::{boot, system};

use crate::util::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemoryMapType {
    Free,
    Device,
    Reserved,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryMapEntry {
    pub physical_address_start: usize,
    pub page_count: usize,
    pub memory_type: MemoryMapType,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryInfo {
    pub memory_size: usize,
    pub memory_map_count: u16,
    pub memory_map: *mut MemoryMapEntry,
}

// make memory info from uefi memory map

static mut MEMORY_MAP_BUFFER: [MemoryMapEntry; 256] = [MemoryMapEntry {
    physical_address_start: 0,
    page_count: 0,
    memory_type: MemoryMapType::Reserved,
}; 256];

pub fn make_memory_info() -> BootResult<MemoryInfo> {
    let mut memory_map_count: u16 = 0;

    uefi::boot::memory_map(uefi::mem::memory_map::MemoryType::LOADER_DATA).and_then(|buffer| {
        buffer
            .entries()
            .enumerate()
            .try_for_each(|(i, entry)| unsafe {
                // add or merge entry logic
                let new_entry = MemoryMapEntry {
                    physical_address_start: entry.phys_start as usize,
                    page_count: entry.page_count as usize,
                    memory_type: match entry.ty {
                        MemoryType::CONVENTIONAL
                        | MemoryType::ACPI_RECLAIM
                        | MemoryType::PERSISTENT_MEMORY => MemoryMapType::Free,
                        MemoryType::RESERVED
                        | MemoryType::BOOT_SERVICES_CODE
                        | MemoryType::BOOT_SERVICES_DATA
                        | MemoryType::RUNTIME_SERVICES_CODE
                        | MemoryType::RUNTIME_SERVICES_DATA
                        | MemoryType::UNUSABLE
                        | MemoryType::ACPI_NON_VOLATILE
                        | MemoryType::PAL_CODE => MemoryMapType::Reserved,
                        MemoryType::LOADER_CODE
                        | MemoryType::LOADER_DATA
                        | MemoryType::MMIO
                        | MemoryType::MMIO_PORT_SPACE
                        | _ => MemoryMapType::Device,
                    },
                };
                let last_entry = if memory_map_count > 0 {
                    &mut MEMORY_MAP_BUFFER[(memory_map_count - 1) as usize]
                } else {
                    core::ptr::null_mut()
                };
                if !last_entry.is_null()
                    && (*last_entry).memory_type == new_entry.memory_type
                    && ((*last_entry).physical_address_start
                        + ((*last_entry).page_count * EFI_PAGE_SIZE)
                        == new_entry.physical_address_start)
                {
                    (*last_entry).page_count += new_entry.page_count;
                } else {
                    MEMORY_MAP_BUFFER[memory_map_count as usize] = new_entry;
                    memory_map_count += 1;
                }

                // making "gap" entry logic (w (1 << 46) max address)
                if i + 1 < buffer.entries().len() {
                    let next_entry = buffer
                        .entries()
                        .nth(i + 1)
                        .ok_or(uefi_error(uefi::Status::INVALID_PARAMETER))?;
                    let last_processed_addr =
                        entry.phys_start as usize + (entry.page_count as usize * EFI_PAGE_SIZE);
                    if next_entry.phys_start as usize > last_processed_addr {
                        let gap_entry = MemoryMapEntry {
                            physical_address_start: last_processed_addr,
                            page_count: (next_entry.phys_start as usize - last_processed_addr)
                                / EFI_PAGE_SIZE,
                            memory_type: MemoryMapType::Device,
                        };
                        let last_entry = &mut MEMORY_MAP_BUFFER[(memory_map_count - 1) as usize];
                        if last_entry.memory_type == gap_entry.memory_type
                            && (last_entry.physical_address_start
                                + (last_entry.page_count * EFI_PAGE_SIZE)
                                == gap_entry.physical_address_start)
                        {
                            last_entry.page_count += gap_entry.page_count;
                        } else {
                            MEMORY_MAP_BUFFER[memory_map_count as usize] = gap_entry;
                            memory_map_count += 1;
                        }
                    }
                } else {
                    let max_address = (1usize) << 46;
                    let last_processed_addr =
                        entry.phys_start as usize + (entry.page_count as usize * EFI_PAGE_SIZE);
                    if last_processed_addr < max_address {
                        let final_gap_entry = MemoryMapEntry {
                            physical_address_start: last_processed_addr,
                            page_count: (max_address - last_processed_addr) / EFI_PAGE_SIZE,
                            memory_type: MemoryMapType::Device,
                        };
                        let last_entry = &mut MEMORY_MAP_BUFFER[(memory_map_count - 1) as usize];
                        if last_entry.memory_type == final_gap_entry.memory_type
                            && (last_entry.physical_address_start
                                + (last_entry.page_count * EFI_PAGE_SIZE)
                                == final_gap_entry.physical_address_start)
                        {
                            last_entry.page_count += final_gap_entry.page_count;
                        } else {
                            MEMORY_MAP_BUFFER[memory_map_count as usize] = final_gap_entry;
                            memory_map_count += 1;
                        }
                    }
                }
                Ok(())
            })
    })?;

    Ok(MemoryInfo {
        memory_size: 0, // maybe unused
        memory_map_count,
        #[allow(static_mut_refs)]
        memory_map: unsafe { MEMORY_MAP_BUFFER.as_mut_ptr() },
    })
}
