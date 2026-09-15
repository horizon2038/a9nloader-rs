const PAGE_SIZE: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryMapType {
    Free,
    Device,
    Reserved,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryMapEntry {
    pub physical_address_start: usize,
    pub page_count: usize,
    pub memory_type: MemoryMapType,
}

impl MemoryMapEntry {
    pub const EMPTY: Self = Self {
        physical_address_start: 0,
        page_count: 0,
        memory_type: MemoryMapType::Reserved,
    };

    fn end(self) -> Result<usize, MapError> {
        if self.physical_address_start % PAGE_SIZE != 0 {
            return Err(MapError::InvalidRange);
        }
        self.page_count
            .checked_mul(PAGE_SIZE)
            .and_then(|bytes| self.physical_address_start.checked_add(bytes))
            .ok_or(MapError::InvalidRange)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    InvalidRange,
    Overlap,
    Capacity,
}

// Input must be sorted by physical address. Reject overlaps rather than publishing
// conflicting Free/Device capabilities for the same physical memory.
pub fn build_memory_map(
    entries: impl IntoIterator<Item = MemoryMapEntry>,
    output: &mut [MemoryMapEntry],
    gap_limit: usize,
) -> Result<usize, MapError> {
    if gap_limit % PAGE_SIZE != 0 {
        return Err(MapError::InvalidRange);
    }
    let mut count = 0;
    let mut previous_end = None;
    for entry in entries {
        if entry.page_count == 0 {
            continue;
        }
        let end = entry.end()?;
        if let Some(start) = previous_end {
            if entry.physical_address_start < start {
                return Err(MapError::Overlap);
            }
            append_gap(output, &mut count, start, entry.physical_address_start)?;
        }
        append(output, &mut count, entry)?;
        previous_end = Some(end);
    }
    let end = previous_end.ok_or(MapError::InvalidRange)?;
    if end < gap_limit {
        append_gap(output, &mut count, end, gap_limit)?;
    }
    Ok(count)
}

fn append_gap(
    output: &mut [MemoryMapEntry],
    count: &mut usize,
    start: usize,
    end: usize,
) -> Result<(), MapError> {
    if start == end {
        return Ok(());
    }
    append(
        output,
        count,
        MemoryMapEntry {
            physical_address_start: start,
            page_count: (end - start) / PAGE_SIZE,
            memory_type: MemoryMapType::Device,
        },
    )
}

fn append(
    output: &mut [MemoryMapEntry],
    count: &mut usize,
    entry: MemoryMapEntry,
) -> Result<(), MapError> {
    if *count > 0 {
        let last = &mut output[*count - 1];
        if last.memory_type == entry.memory_type && last.end()? == entry.physical_address_start {
            last.page_count = last
                .page_count
                .checked_add(entry.page_count)
                .ok_or(MapError::InvalidRange)?;
            return Ok(());
        }
    }
    *output.get_mut(*count).ok_or(MapError::Capacity)? = entry;
    *count += 1;
    Ok(())
}
