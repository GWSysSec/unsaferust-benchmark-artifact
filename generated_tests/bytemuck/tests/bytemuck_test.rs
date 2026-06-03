use bytemuck::{fill_zeroes, offset_of, write_zeroes, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PaddedRecord {
    tag: u8,
    value: u32,
}

unsafe impl Zeroable for PaddedRecord {}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PacketHeader {
    tag: u8,
    flags: u8,
    length: u16,
    checksum: u32,
}

unsafe impl Zeroable for PacketHeader {}

fn raw_bytes_of<T>(value: &T) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            (value as *const T).cast::<u8>(),
            core::mem::size_of::<T>(),
        )
    }
}

#[test]
fn write_zeroes_clears_fields_and_padding_bytes() {
    let mut record = PaddedRecord {
        tag: 0x7F,
        value: 0xAABB_CCDD,
    };

    assert_eq!(record.tag, 0x7F);
    assert_eq!(record.value, 0xAABB_CCDD);

    write_zeroes(&mut record);

    assert_eq!(record, PaddedRecord { tag: 0, value: 0 });
    assert_eq!(raw_bytes_of(&record).len(), core::mem::size_of::<PaddedRecord>());
    assert!(raw_bytes_of(&record).iter().all(|byte| *byte == 0));
}

#[test]
fn fill_zeroes_clears_every_element_in_a_slice() {
    let mut records = [
        PaddedRecord {
            tag: 1,
            value: 0x1111_2222,
        },
        PaddedRecord {
            tag: 2,
            value: 0x3333_4444,
        },
        PaddedRecord {
            tag: 3,
            value: 0x5555_6666,
        },
    ];

    assert_eq!(records[0].value, 0x1111_2222);
    assert_eq!(records[2].tag, 3);

    fill_zeroes(&mut records);

    assert_eq!(records, [PaddedRecord { tag: 0, value: 0 }; 3]);

    for record in &records {
        assert!(raw_bytes_of(record).iter().all(|byte| *byte == 0));
    }
}

#[test]
fn fill_zeroes_handles_empty_slices_without_changing_neighbors() {
    let mut before = PaddedRecord {
        tag: 9,
        value: 0x1234_5678,
    };
    let mut empty: [PaddedRecord; 0] = [];
    let mut after = PaddedRecord {
        tag: 10,
        value: 0x8765_4321,
    };

    fill_zeroes(&mut empty);

    assert_eq!(
        before,
        PaddedRecord {
            tag: 9,
            value: 0x1234_5678
        }
    );
    assert_eq!(
        after,
        PaddedRecord {
            tag: 10,
            value: 0x8765_4321
        }
    );

    write_zeroes(&mut before);
    write_zeroes(&mut after);

    assert_eq!(before, PaddedRecord { tag: 0, value: 0 });
    assert_eq!(after, PaddedRecord { tag: 0, value: 0 });
}

#[test]
fn offset_of_reports_expected_repr_c_layout() {
    let tag = offset_of!(Zeroable::zeroed(), PacketHeader, tag);
    let flags = offset_of!(Zeroable::zeroed(), PacketHeader, flags);
    let length = offset_of!(Zeroable::zeroed(), PacketHeader, length);
    let checksum = offset_of!(Zeroable::zeroed(), PacketHeader, checksum);

    assert_eq!(tag, 0);
    assert_eq!(flags, 1);
    assert_eq!(length, 2);
    assert_eq!(checksum, 4);
    assert_eq!(core::mem::size_of::<PacketHeader>(), 8);
}

#[test]
fn offset_of_matches_manual_pointer_distance_for_zeroed_value() {
    let record = PaddedRecord::zeroed();

    let base = (&record as *const PaddedRecord).cast::<u8>() as usize;
    let tag_addr = (&record.tag as *const u8) as usize;
    let value_addr = (&record.value as *const u32).cast::<u8>() as usize;

    let tag_offset = offset_of!(Zeroable::zeroed(), PaddedRecord, tag);
    let value_offset = offset_of!(Zeroable::zeroed(), PaddedRecord, value);

    assert_eq!(tag_offset, tag_addr - base);
    assert_eq!(value_offset, value_addr - base);
    assert_eq!(tag_offset, 0);
    assert_eq!(value_offset, 4);
}