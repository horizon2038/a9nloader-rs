use uefi::Error;
use uefi::Status;

pub type BootResult<T> = Result<T, Error>;

pub const EFI_PAGE_SIZE: usize = 4096;
pub const HIGHER_HALF_MASK: usize = 0xFFFF_8000_0000_0000;

#[inline(always)]
pub fn bytes_to_pages(bytes: usize) -> usize {
    (bytes + EFI_PAGE_SIZE - 1) / EFI_PAGE_SIZE
}

#[inline(always)]
pub fn bytes_to_pages_rounded(bytes: usize) -> usize {
    let pages = bytes_to_pages(bytes);
    if pages == 0 { 1 } else { pages }
}

#[inline(always)]
pub fn uefi_error(status: Status) -> Error {
    Error::from(status)
}
