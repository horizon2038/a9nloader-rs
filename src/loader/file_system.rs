use crate::info;

extern crate alloc;
use alloc::vec;

use uefi::fs::FileSystem;
use uefi::{CString16, boot};

use crate::util::BootResult;

pub fn read_entire_file(filepath: &str) -> BootResult<vec::Vec<u8>> {
    info!("Reading file: {}", filepath);
    boot::get_image_file_system(boot::image_handle()).map(FileSystem::new)
        .map_err(|_| crate::util::uefi_error(uefi::Status::INVALID_PARAMETER))
        .and_then(|mut target_fs| {
            CString16::try_from(filepath)
                .map_err(|_| crate::util::uefi_error(uefi::Status::INVALID_PARAMETER))
                // .map(move |cstr16| uefi::fs::Path::new(cstr16))
                .and_then(|path| {
                    let path = uefi::fs::Path::new(path.as_ref());
                    info_file_metadata(path, &mut target_fs)?;
                    target_fs
                        .read(path)
                        .map_err(|_| crate::util::uefi_error(uefi::Status::NOT_FOUND))
                })
        })
}

pub fn info_file_metadata(
    file_path: &uefi::fs::Path,
    file_system: &mut FileSystem,
) -> BootResult<()> {
    file_system
        .metadata(file_path)
        .map_err(|_| crate::util::uefi_error(uefi::Status::NOT_FOUND))
        .map(|metadata| {
            info!(
                "File: {}, Size: {} bytes, created: {}",
                metadata.file_name(),
                metadata.file_size(),
                metadata.create_time()
            );
            
        })
}

pub fn info_file_in_directory(
    directory_path: &uefi::fs::Path,
    file_system: &mut FileSystem,
) -> BootResult<()> {
    file_system
        .read_dir(directory_path)
        .map_err(|_| crate::util::uefi_error(uefi::Status::NOT_FOUND))
        .map(|iter| {
            iter.filter_map(|entry| entry.ok()).for_each(|file_info| {
                info!(
                    "File: {}/{}, Size: {} bytes, created: {}",
                    directory_path.to_cstr16(),
                    file_info.file_name(),
                    file_info.file_size(),
                    file_info.create_time()
                );
            });
            
        })
}
