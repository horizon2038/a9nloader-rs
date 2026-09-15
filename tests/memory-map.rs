#[path = "../src/loader/memory/map.rs"]
mod map;
use map::{MapError, MemoryMapEntry as Entry, MemoryMapType::*};

fn entry(start: usize, end: usize, memory_type: map::MemoryMapType) -> Entry {
    Entry {
        physical_address_start: start,
        page_count: (end - start) / 4096,
        memory_type,
    }
}

fn build(input: &[Entry], limit: usize) -> Vec<Entry> {
    let mut output = [Entry::EMPTY; 256];
    let count = map::build_memory_map(input.iter().copied(), &mut output, limit).unwrap();
    output[..count].to_vec()
}

#[test]
fn four_gib_free_from_zero_is_preserved() {
    let input = [entry(0, 1 << 32, Free)];
    assert_eq!(build(&input, 1 << 32), input);
}

#[test]
fn reserved_page_zero_is_preserved_too() {
    let input = [entry(0, 4096, Reserved), entry(4096, 0x6000, Free)];
    assert_eq!(build(&input, 0x6000), input);
}

#[test]
fn firmware_tail_entries_do_not_create_holes_over_ram() {
    // The real firmware returned low Reserved entries after high Free RAM.
    let mut input = [
        entry(0, 0x6000, Free),
        entry(0x6000, 0x7000, Reserved),
        entry(0x7000, 0x9f000, Free),
        entry(0x9f000, 0xa0000, Reserved),
        entry(0x100000, 0x8ceff000, Free),
        entry(0x100000000, 0x140000000, Free),
        entry(0x140000000, 0x14008b000, Device),
        entry(0x14008b000, 0x26e000000, Free),
        entry(0xa0000, 0x100000, Reserved),
        entry(0x8ceff000, 0x90000000, Reserved),
    ];
    // make_memory_info calls UEFI MemoryMapMut::sort before the builder.
    input.sort_unstable_by_key(|e| e.physical_address_start);
    let output = build(&input, 1 << 46);
    for region in input.iter().filter(|e| e.memory_type == Free) {
        assert!(output.contains(region));
        let end = region.physical_address_start + region.page_count * 4096;
        for device in output.iter().filter(|e| e.memory_type == Device) {
            let device_end = device.physical_address_start + device.page_count * 4096;
            assert!(
                device_end <= region.physical_address_start || device.physical_address_start >= end
            );
        }
    }
    assert_eq!(output.last(), Some(&entry(0x26e000000, 1 << 46, Device)));
}

#[test]
fn gaps_and_adjacent_regions_merge_without_crossing_types() {
    let input = [
        entry(0, 0x2000, Free),
        entry(0x2000, 0x3000, Free),
        entry(0x4000, 0x5000, Device),
        entry(0x6000, 0x7000, Reserved),
    ];
    assert_eq!(
        build(&input, 0x9000),
        [
            entry(0, 0x3000, Free),
            entry(0x3000, 0x6000, Device),
            entry(0x6000, 0x7000, Reserved),
            entry(0x7000, 0x9000, Device),
        ]
    );
}

#[test]
fn overlapping_and_unsorted_entries_are_rejected() {
    let mut output = [Entry::EMPTY; 8];
    for input in [
        [entry(0, 0x4000, Free), entry(0x3000, 0x5000, Reserved)],
        [entry(0x4000, 0x5000, Free), entry(0, 0x1000, Free)],
        [entry(0, 0x2000, Free), entry(0, 0x2000, Free)],
    ] {
        assert_eq!(
            map::build_memory_map(input, &mut output, 0x10000),
            Err(MapError::Overlap)
        );
    }
}

#[test]
fn invalid_ranges_are_rejected() {
    let mut output = [Entry::EMPTY; 8];
    for input in [
        Entry {
            physical_address_start: 1,
            page_count: 1,
            memory_type: Free,
        },
        Entry {
            physical_address_start: 0,
            page_count: usize::MAX,
            memory_type: Free,
        },
        Entry {
            physical_address_start: usize::MAX & !4095,
            page_count: 1,
            memory_type: Free,
        },
    ] {
        assert_eq!(
            map::build_memory_map([input], &mut output, 0x10000),
            Err(MapError::InvalidRange)
        );
    }
    assert_eq!(
        map::build_memory_map([entry(0, 4096, Free)], &mut output, 1),
        Err(MapError::InvalidRange)
    );
}

#[test]
fn capacity_error_does_not_overwrite_neighboring_memory() {
    let sentinel = entry(0xdead000, 0xdeae000, Reserved);
    let mut output = [sentinel; 3];
    assert_eq!(
        map::build_memory_map([entry(0, 4096, Free)], &mut output[1..2], 8192),
        Err(MapError::Capacity)
    );
    assert_eq!(output[0], sentinel);
    assert_eq!(output[2], sentinel);
    assert_eq!(
        map::build_memory_map([entry(0, 4096, Free)], &mut [], 4096),
        Err(MapError::Capacity)
    );
}

#[test]
fn merging_still_works_at_capacity() {
    let mut output = [Entry::EMPTY; 1];
    let input = [entry(0, 4096, Device), entry(8192, 12288, Device)];
    assert_eq!(map::build_memory_map(input, &mut output, 16384), Ok(1));
    assert_eq!(output[0], entry(0, 16384, Device));
}

#[test]
fn empty_maps_are_rejected_and_empty_entries_are_ignored() {
    let mut output = [Entry::EMPTY; 4];
    assert_eq!(
        map::build_memory_map([], &mut output, 4096),
        Err(MapError::InvalidRange)
    );
    assert_eq!(
        map::build_memory_map([Entry::EMPTY], &mut output, 4096),
        Err(MapError::InvalidRange)
    );
    assert_eq!(
        build(&[Entry::EMPTY, entry(0, 4096, Free)], 4096),
        [entry(0, 4096, Free)]
    );
}

#[test]
fn memory_above_gap_limit_is_not_truncated() {
    let input = [entry(1 << 46, (1 << 46) + 4096, Free)];
    assert_eq!(build(&input, 1 << 46), input);
}

#[test]
fn boot_abi_layout_is_unchanged() {
    assert_eq!(core::mem::size_of::<map::MemoryMapType>(), 4);
    assert_eq!(core::mem::size_of::<Entry>(), 24);
    assert_eq!(core::mem::offset_of!(Entry, physical_address_start), 0);
    assert_eq!(core::mem::offset_of!(Entry, page_count), 8);
    assert_eq!(core::mem::offset_of!(Entry, memory_type), 16);
}
