// Direct tests of the `WVMMemory` API (the `load_*`/`store_*` methods).
use w8_core::vm::memory::WVMMemory;

// Regression test: an address near `usize::MAX` must not overflow in
// `address + size` (a debug build used to panic with "attempt to add
// with overflow" instead of returning `None`).
#[test]
fn huge_addresses_return_none_instead_of_overflowing() {
    let mem = WVMMemory::new(64);

    assert_eq!(mem.load_u8(usize::MAX), None);
    assert_eq!(mem.load_u16(usize::MAX), None);
    assert_eq!(mem.load_u32(usize::MAX), None);
    assert_eq!(mem.load_u64(usize::MAX), None);
    assert_eq!(mem.load_i8(usize::MAX), None);
    assert_eq!(mem.load_i16(usize::MAX), None);
    assert_eq!(mem.load_i32(usize::MAX), None);
    assert_eq!(mem.load_i64(usize::MAX), None);
    assert_eq!(mem.load_f32(usize::MAX), None);
    assert_eq!(mem.load_f64(usize::MAX), None);

    assert_eq!(mem.load_u16(usize::MAX - 1), None);
    assert_eq!(mem.load_u64(usize::MAX - 7), None);
}

#[test]
fn stores_with_huge_addresses_return_none() {
    let mut mem = WVMMemory::new(64);

    assert_eq!(mem.store_u8(usize::MAX, 1), None);
    assert_eq!(mem.store_u16(usize::MAX, 1), None);
    assert_eq!(mem.store_u32(usize::MAX, 1), None);
    assert_eq!(mem.store_u64(usize::MAX, 1), None);
    assert_eq!(mem.store_i16(usize::MAX, 1), None);
    assert_eq!(mem.store_i32(usize::MAX, 1), None);
    assert_eq!(mem.store_i64(usize::MAX, 1), None);
    assert_eq!(mem.store_f32(usize::MAX, 1.0), None);
    assert_eq!(mem.store_f64(usize::MAX, 1.0), None);
}

#[test]
fn loads_work_up_to_the_last_valid_address() {
    let mut mem = WVMMemory::new(8);
    mem.store_u64(0, u64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8]));

    assert_eq!(mem.load_u16(6), Some(u16::from_le_bytes([7, 8])));
    assert_eq!(mem.load_u32(4), Some(u32::from_le_bytes([5, 6, 7, 8])));
    assert_eq!(
        mem.load_u64(0),
        Some(u64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8]))
    );
    assert_eq!(mem.load_u64(1), None);
}
