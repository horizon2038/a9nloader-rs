mod elf;
pub use elf::*;

mod loader;
pub use loader::*;

mod file_system;
pub use file_system::*;

mod memory;
pub use memory::*;

mod boot_info;
pub use boot_info::*;

mod frame_buffer_info;
pub use frame_buffer_info::*;

use crate::info;
use crate::util::*;

const KERNEL_PATH: &str = r"\kernel\kernel.elf";
const INIT_PATH: &str = r"\kernel\init.elf";

pub fn run() -> BootResult<()> {
    info!("Starting load a kernel...");
    let mut kernel_entry_point: usize = 0;

    read_entire_file(KERNEL_PATH).and_then(|kernel_bytes| {
        parse_elf(&kernel_bytes)
            .and_then(|kernel_elf| load_kernel_at_physical_address(&kernel_elf, &kernel_bytes))
            .map(|entry_point| {
                info!(
                    "Kernel loaded successfully at entry point: 0x{:016x}",
                    entry_point
                );
                kernel_entry_point = entry_point;
            })
            .and_then(|_| reserve_ap_trampoline())
            .and_then(|_| read_entire_file(INIT_PATH))
            .and_then(|init_bytes| {
                parse_elf(&init_bytes)
                    .and_then(|init_elf| load_init_at_anywhere(&init_elf, &init_bytes))
                    .map(|fetched_init_image_info| {
                        info!(
                            "Init loaded successfully at entry point: 0x{:016x}",
                            fetched_init_image_info.entry_point_virtual_address
                        );
                        info!(
                            "Init image: loaded at 0x{:016x}, pages: {}, entry point: 0x{:016x}",
                            fetched_init_image_info.loaded_address,
                            fetched_init_image_info.init_image_pages,
                            fetched_init_image_info.entry_point_virtual_address
                        );
                        unsafe { BOOT_INFO.init_image_info = fetched_init_image_info };

                        core::mem::forget(init_bytes);

                        info!("Init image info prepared.");
                    })
            })
            .and_then(|_| {
                info!("Preparing memory info...");
                make_memory_info().map(|memory_info| {
                    for i in 0..memory_info.memory_map_count as usize {
                        let entry = unsafe { &*memory_info.memory_map.add(i) };
                        info!(
                            "Memory Map Entry {}: Address: 0x{:016x}, Pages: {}, Type: {:?}",
                            i, entry.physical_address_start, entry.page_count, entry.memory_type
                        );
                    }
                    unsafe { BOOT_INFO.memory_info = memory_info };
                    
                })
            })
            .map(|_| {
                // arch_info[0]: rsdp
                unsafe {
                    BOOT_INFO.arch_info[0] = find_rsdp_address();
                    info!("Loading finished. Preparing to jump to kernel...");
                    let _ = uefi::boot::exit_boot_services(Some(
                        uefi::mem::memory_map::MemoryType::LOADER_DATA,
                    ));

                    // jump to kernel with BOOT_INFO address
                    // sysv abi
                    let kernel_entry: extern "sysv64" fn(*const BootInfo) -> ! =
                        core::mem::transmute(kernel_entry_point);

                    #[allow(static_mut_refs)]
                    kernel_entry(&BOOT_INFO as *const BootInfo);
                }
                #[allow(unreachable_code)]
                ()
            })
    })
}

fn find_rsdp_address() -> usize {
    uefi::system::with_config_table(|table| {
        let mut rsdp_acpi1: usize = 0;
        let mut rsdp_acpi2: usize = 0;

        for entry in table.iter() {
            if entry.guid == uefi::table::cfg::ACPI2_GUID {
                rsdp_acpi2 = entry.address as usize;
            } else if entry.guid == uefi::table::cfg::ACPI_GUID {
                rsdp_acpi1 = entry.address as usize;
            }
        }

        let rsdp = if rsdp_acpi2 != 0 {
            rsdp_acpi2
        } else {
            rsdp_acpi1
        };

        if rsdp != 0 {
            info!(
                "Chosen RSDP: 0x{:016x} ({})",
                rsdp,
                if rsdp == rsdp_acpi2 {
                    "ACPI 2.0+"
                } else {
                    "ACPI 1.0"
                }
            );
        }

        rsdp
    })
}
