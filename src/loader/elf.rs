use crate::{debug, error, info, warn};

use crate::util::BootResult;
use crate::util::uefi_error;

use uefi::Status;
use xmas_elf::ElfFile;
use xmas_elf::sections::{SectionData, SectionHeader, ShType};
use xmas_elf::symbol_table::{self, Entry};

pub fn parse_elf(bytes: &[u8]) -> BootResult<ElfFile<'_>> {
    xmas_elf::ElfFile::new(bytes).map_err(|e| {
        error!("Failed to parse ELF file: {}", e);
        uefi_error(Status::LOAD_ERROR)
    })
}

pub fn find_address_from_symbol_name(elf: &ElfFile, symbol_name: &str) -> BootResult<usize> {
    // read the section headers (table)
    for section_header in elf.section_iter() {
        // search for symbol table section (.symtab)
        if section_header.get_type() != Ok(ShType::SymTab) {
            continue;
        }

        // search for string table section (.strtab)
        let string_table = match lookup_string_table(elf, &section_header) {
            Some(table) => table,
            None => {
                debug!(
                    "Failed to find string table for section: {:?}",
                    section_header
                );
                continue;
            }
        };

        // search symbol table entry from the symbol table section (.symtab) and string table
        // (.strtab)
        if let Some(address) =
            lookup_address_in_symbol_table(elf, &section_header, string_table, symbol_name)
        {
            return Ok(address);
        } else {
            debug!(
                "Symbol '{}' not found in section: {:?}",
                symbol_name, section_header
            );
        }
    }

    error!("Failed to read symbol table");
    Err(uefi_error(Status::NOT_FOUND))
}

fn lookup_string_table<'a>(
    elf: &ElfFile<'a>,
    section_header: &SectionHeader<'a>,
) -> Option<&'a [u8]> {
    match section_header.link() {
        string_table_section_index => {
            let section_header_string = elf.section_header(string_table_section_index as u16);
            return match section_header_string {
                Ok(sh) => match sh.get_data(elf) {
                    Ok(SectionData::StrArray(string_table_raw)) => Some(string_table_raw),
                    _ => None,
                },
                Err(e) => {
                    debug!("Failed to get string table section: {}", e);
                    None
                }
            };
        }
    };
}

fn lookup_address_in_symbol_table(
    elf: &ElfFile,
    section_header: &SectionHeader,
    string_table: &[u8],
    symbol_name: &str,
) -> Option<usize> {
    return match section_header.get_data(elf) {
        Ok(SectionData::SymbolTable64(entries)) => {
            for entry in entries {
                if compare_from_index(string_table, entry.name() as usize, symbol_name) {
                    // found the symbol
                    let address = entry.value() as usize;
                    info!("Found symbol '{}' at address: {:#x}", symbol_name, address);
                    return Some(address);
                };
            }

            None
        }
        _ => None,
    };
}

fn compare_from_index(string_table: &[u8], index: usize, symbol_name: &str) -> bool {
    let target_bytes = symbol_name.as_bytes();
    let target_length = target_bytes.len();

    if string_table.len() < index + target_length {
        return false; // Out of bounds
    }

    let slice_to_compare = &string_table[index..index + target_length];

    slice_to_compare == target_bytes
}
