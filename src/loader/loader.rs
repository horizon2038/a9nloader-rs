use crate::{debug, error, info, warn};

use core::ptr::{copy_nonoverlapping, write_bytes};

use crate::loader::elf;
use crate::util::*;
use uefi::boot::{self, MemoryType};
use xmas_elf::{
    ElfFile,
    program::{ProgramHeader, Type as ProgramHeaderType},
};

pub const AP_TRAMPOLINE_BASE: usize = 0x6000;

pub struct InitImageInfo {
    pub loaded_address: usize,
    pub init_image_pages: usize,
    pub entry_point_virtual_address: usize,
    pub init_info_virtual_address: usize,
    pub init_ipc_buffer_virtual_address: usize,
}

pub fn load_kernel_at_physical_address(
    kernel_elf: &ElfFile,
    kernel_bytes: &[u8],
) -> BootResult<usize> {
    info!("Loading kernel ...");
    kernel_elf
        .program_iter()
        .filter(filter_program_header_load)
        .try_for_each(|program_header| allocate_segment_at_exact_physical_address(&program_header))
        .and_then(|_| {
            kernel_elf
                .program_iter()
                .filter(filter_program_header_load)
                .try_for_each(|program_header| {
                    copy_segment_to_physical_address(&program_header, kernel_bytes, 0)
                })
        })
        .and_then(|_| {
            let entry_point = kernel_elf.header.pt2.entry_point() as usize;
            info!("Kernel entry point: 0x{:016x}", entry_point);
            Ok(entry_point)
        })
}

#[inline]
fn filter_program_header_load(program_header: &ProgramHeader) -> bool {
    program_header.get_type() != Ok(ProgramHeaderType::Load)
}

fn allocate_segment_at_exact_physical_address(program_header: &ProgramHeader) -> BootResult<()> {
    let physical_address = (program_header.physical_addr() as usize) & !HIGHER_HALF_MASK;
    let memory_size = program_header.mem_size() as usize;
    let pages = bytes_to_pages(memory_size);

    if pages == 0 {
        // warn!("Program header with zero memory size: {:?}", program_header);
        return Ok(());
    }

    boot::allocate_pages(
        boot::AllocateType::Address(physical_address as u64),
        MemoryType::RESERVED,
        pages,
    )
    .map_err(|e| e.status())
    .map_err(crate::util::uefi_error)
    .map(|_| {
        debug!(
            "Alloc segment at [0x{:016x}, 0x{:016x}] with {} pages",
            physical_address,
            physical_address + memory_size,
            pages
        );
    })
}

fn copy_segment_to_physical_address(
    program_header: &ProgramHeader,
    image: &[u8],
    physical_offset: usize,
) -> BootResult<()> {
    if !filter_program_header_load(program_header) {
        return Ok(());
    }

    let file_size = program_header.file_size() as usize;
    let memory_size = program_header.mem_size() as usize;
    let file_offset = program_header.offset() as usize;

    let mut physical_address = program_header.physical_addr() as usize + physical_offset;
    if file_size == 0 {
        physical_address &= !HIGHER_HALF_MASK; // Ensure physical address is in lower half
    }

    if file_size > 0 {
        let source = &image[file_offset..file_offset + file_size];
        unsafe {
            copy_nonoverlapping(source.as_ptr(), physical_address as *mut u8, file_size);
        }
    }

    // clear bss
    if memory_size > file_size {
        let bss_length = memory_size - file_size;
        unsafe {
            write_bytes((physical_address + file_size) as *mut u8, 0, bss_length);
        }
    }

    Ok(())
}

pub fn load_init_at_anywhere(init_elf: &ElfFile, init_bytes: &[u8]) -> BootResult<InitImageInfo> {
    info!("Loading init ...");

    let (span_start, span_end) = calculate_load_span_physical_address(init_elf);
    let total_bytes = span_end - span_start;
    let total_pages = bytes_to_pages_rounded(total_bytes);

    let mut base = span_start;
    uefi::boot::allocate_pages(
        uefi::boot::AllocateType::AnyPages,
        MemoryType::RESERVED,
        total_pages,
    )
    .map(|address| {
        let address_raw: usize = address.as_ptr().addr();
        base = address_raw;
        address_raw
    })
    .map_err(|e| {
        error!("Failed to allocate pages for init: {}", e);
        e
    })
    .and_then(|_| {
        info!(
            "Init base address: 0x{:016x}, total pages: 0x{:x}",
            base, total_pages
        );
        init_elf
            .program_iter()
            .filter(filter_program_header_load)
            .try_for_each(|program_header| {
                copy_segment_to_physical_address(&program_header, init_bytes, base)
            })
    })
    .and_then(|_| {
        let entry_virtual_address = init_elf.header.pt2.entry_point() as usize;
        let init_info_virtual_address =
            elf::find_address_from_symbol_name(init_elf, "__init_info_start")?;
        let init_ipc_buffer_virtual_address =
            elf::find_address_from_symbol_name(init_elf, "__init_ipc_buffer_start")?;
        info!("Init entry point: 0x{:016x}", entry_virtual_address);
        info!(
            "Init info virtual address: 0x{:016x}",
            init_info_virtual_address
        );
        info!(
            "Init IPC buffer virtual address: 0x{:016x}",
            init_ipc_buffer_virtual_address
        );

        Ok(InitImageInfo {
            loaded_address: base,
            init_image_pages: total_pages,
            entry_point_virtual_address: entry_virtual_address,
            init_info_virtual_address,
            init_ipc_buffer_virtual_address,
        })
    })
}

fn calculate_load_span_physical_address(elf: &ElfFile) -> (usize, usize) {
    let mut start = usize::MAX;
    let mut end = 0usize;

    for program_header in elf.program_iter() {
        if !filter_program_header_load(&program_header) {
            continue;
        }

        let physical_start = program_header.physical_addr() as usize;
        let memory_size = program_header.mem_size() as usize;

        if physical_start < start {
            start = physical_start;
        }
        if physical_start + memory_size > end {
            end = physical_start + memory_size;
        }
    }

    (start, end)
}

pub fn reserve_ap_trampoline() -> BootResult<()> {
    info!(
        "Reserving AP trampoline at 0x{:016x}...",
        AP_TRAMPOLINE_BASE
    );

    let try_types = [MemoryType::UNUSABLE, MemoryType::RESERVED];
    for try_type in try_types {
        let result = boot::allocate_pages(
            boot::AllocateType::Address(AP_TRAMPOLINE_BASE as u64),
            try_type,
            1,
        );

        match result {
            Ok(_) => {
                info!("Reserved AP trampoline at 0x{:016x}", AP_TRAMPOLINE_BASE);
                return Ok(());
            }
            Err(_) => {
                warn!(
                    "Failed to reserve AP trampoline at 0x{:016x}: {}",
                    AP_TRAMPOLINE_BASE,
                    result.unwrap_err()
                );
            }
        }
    }

    Err(uefi_error(uefi::Status::OUT_OF_RESOURCES))
}
