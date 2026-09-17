use uefi::boot::MemoryType;

use super::MemoryMapType;

pub fn classify(memory_type: MemoryType) -> MemoryMapType {
    match memory_type {
        MemoryType::CONVENTIONAL | MemoryType::PERSISTENT_MEMORY => MemoryMapType::Free,
        // ACPI tables may reside in either region. Device capabilities permit
        // user-space discovery without making firmware data allocator RAM.
        // In particular, NVS must retain its contents, not be reclaimed/cleared.
        MemoryType::ACPI_RECLAIM | MemoryType::ACPI_NON_VOLATILE => MemoryMapType::Device,
        MemoryType::RESERVED
        | MemoryType::BOOT_SERVICES_CODE
        | MemoryType::BOOT_SERVICES_DATA
        | MemoryType::RUNTIME_SERVICES_CODE
        | MemoryType::RUNTIME_SERVICES_DATA
        | MemoryType::UNUSABLE
        | MemoryType::PAL_CODE => MemoryMapType::Reserved,
        // Preserve the existing policy for loader/MMIO/unknown types.
        _ => MemoryMapType::Device,
    }
}
