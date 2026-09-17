# Memory map regression tests

The map builder is independent of UEFI services and can be tested on the host
without the loader's UEFI allocator or target configuration:

```sh
test_dir=$(mktemp -d)
rustc --edition 2024 --test tests/memory-map.rs -o "$test_dir/memory-map"
"$test_dir/memory-map"
```

Production `make_memory_info` first sorts the firmware map in place using
`MemoryMapMut::sort`, then passes it to this builder. Tests cover the real-machine
ordering pattern (low Reserved entries after high RAM), preservation of Free RAM
starting at PA 0, type boundaries, gaps, overlapping/malformed ranges, capacity
limits and the unchanged C ABI layout. Explicitly conflicting firmware ranges
are rejected, not assigned competing Free and Device classifications.

The Nanami workspace also tests the production UEFI classification and ACPI
walker together (`tests/native-performance/acpi-tests.rs`), using `uefi` without
its firmware allocator/panic-handler features. ACPI reclaim and NVS are published
as Device capabilities, not ordinary Free RAM; general Reserved/runtime/boot
services remain excluded. This covers NVS tables, content preservation, mapping
failure diagnostics, checksums and RSDT/page-boundary cases.
