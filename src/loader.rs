mod elf;
pub use elf::*;

mod loader;
pub use loader::*;

mod file_system;
pub use file_system::*;

mod memory;
pub use memory::*;

use crate::util::*;
use crate::{debug, error, info, warn};

const KERNEL_PATH: &str = r"\kernel\kernel.elf";
const INIT_PATH: &str = r"\kernel\init.elf";

pub fn run() -> BootResult<()> {
    info!("Starting load a kernel...");
    let mut kernel_entry_point: usize = 0;
    let mut init_entry_point: usize = 0;

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
                    .map(|init_image_info| {
                        info!(
                            "Init loaded successfully at entry point: 0x{:016x}",
                            init_image_info.entry_point_virtual_address
                        );
                        info!(
                            "Init image: loaded at 0x{:016x}, pages: {}, entry point: 0x{:016x}",
                            init_image_info.loaded_address,
                            init_image_info.init_image_pages,
                            init_image_info.entry_point_virtual_address
                        );
                    })
            })
    })
}
