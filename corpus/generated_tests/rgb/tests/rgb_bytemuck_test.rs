use rgb::bytemuck;

#[test]
fn numeric_value_round_trips_through_byte_views_and_casts() {
    let mut value = 0x1122_3344u32;
    let native = value.to_ne_bytes();

    let readonly_bytes = bytemuck::bytes_of(&value);
    assert_eq!(readonly_bytes, native.as_slice());

    {
        let mutable_bytes = bytemuck::bytes_of_mut(&mut value);
        assert_eq!(mutable_bytes.len(), 4);
        mutable_bytes.copy_from_slice(&0xA1B2_C3D4u32.to_ne_bytes());
    }
    assert_eq!(value, 0xA1B2_C3D4u32);

    let array_from_cast: [u8; 4] = bytemuck::cast::<u32, [u8; 4]>(value);
    assert_eq!(array_from_cast, value.to_ne_bytes());

    let value_from_array: u32 = bytemuck::cast::<[u8; 4], u32>(array_from_cast);
    assert_eq!(value_from_array, value);

    let bytes_ref: &[u8; 4] = bytemuck::cast_ref::<u32, [u8; 4]>(&value);
    assert_eq!(*bytes_ref, value.to_ne_bytes());

    {
        let bytes_mut: &mut [u8; 4] = bytemuck::cast_mut::<u32, [u8; 4]>(&mut value);
        *bytes_mut = 0x0102_0304u32.to_ne_bytes();
    }
    assert_eq!(value, 0x0102_0304u32);

    let try_bytes: [u8; 4] = bytemuck::try_cast::<u32, [u8; 4]>(value).unwrap();
    assert_eq!(try_bytes, value.to_ne_bytes());

    let try_bytes_ref: &[u8; 4] = bytemuck::try_cast_ref::<u32, [u8; 4]>(&value).unwrap();
    assert_eq!(*try_bytes_ref, value.to_ne_bytes());

    {
        let try_bytes_mut: &mut [u8; 4] =
            bytemuck::try_cast_mut::<u32, [u8; 4]>(&mut value).unwrap();
        *try_bytes_mut = 0xDEAD_BEEFu32.to_ne_bytes();
    }
    assert_eq!(value, 0xDEAD_BEEFu32);

    assert!(bytemuck::try_cast::<u16, u32>(0x1234u16).is_err());
}

#[test]
fn slices_can_be_reinterpreted_and_edited_as_bytes() {
    let words = [0x1122u16, 0x3344, 0x5566, 0x7788];
    let bytes: &[u8] = bytemuck::cast_slice::<u16, u8>(&words);

    let expected: Vec<u8> = words
        .iter()
        .flat_map(|word| word.to_ne_bytes())
        .collect();
    assert_eq!(bytes, expected.as_slice());
    assert_eq!(bytes.len(), words.len() * std::mem::size_of::<u16>());

    let back_to_words: &[u16] = bytemuck::try_cast_slice::<u8, u16>(bytes).unwrap();
    assert_eq!(back_to_words, words.as_slice());

    let mut editable = [0u16; 3];
    {
        let editable_bytes: &mut [u8] = bytemuck::cast_slice_mut::<u16, u8>(&mut editable);
        assert_eq!(editable_bytes.len(), 6);
        editable_bytes[0..2].copy_from_slice(&0xCAFEu16.to_ne_bytes());
        editable_bytes[2..4].copy_from_slice(&0xBABEu16.to_ne_bytes());
        editable_bytes[4..6].copy_from_slice(&0x1234u16.to_ne_bytes());
    }
    assert_eq!(editable, [0xCAFE, 0xBABE, 0x1234]);

    {
        let editable_as_bytes: &mut [u8] =
            bytemuck::try_cast_slice_mut::<u16, u8>(&mut editable).unwrap();
        editable_as_bytes[0..2].copy_from_slice(&0x0F0Eu16.to_ne_bytes());
    }
    assert_eq!(editable[0], 0x0F0E);

    let odd_byte_count = [1u16, 2, 3];
    assert!(bytemuck::try_cast_slice::<u16, u32>(&odd_byte_count).is_err());
}

#[test]
fn bytes_to_values_work_for_aligned_and_unaligned_inputs() {
    let source = 0x7856_3412u32;
    let source_bytes = bytemuck::bytes_of(&source);

    let from_bytes_ref: &u32 = bytemuck::from_bytes::<u32>(source_bytes);
    assert_eq!(*from_bytes_ref, source);

    let try_from_bytes_ref: &u32 = bytemuck::try_from_bytes::<u32>(source_bytes).unwrap();
    assert_eq!(*try_from_bytes_ref, source);

    let mut mutable_source = 0u32;
    {
        let mutable_bytes = bytemuck::bytes_of_mut(&mut mutable_source);
        let from_bytes_mut_ref: &mut u32 = bytemuck::from_bytes_mut::<u32>(mutable_bytes);
        *from_bytes_mut_ref = 0xAABB_CCDDu32;
    }
    assert_eq!(mutable_source, 0xAABB_CCDDu32);

    {
        let mutable_bytes = bytemuck::bytes_of_mut(&mut mutable_source);
        let try_from_bytes_mut_ref: &mut u32 =
            bytemuck::try_from_bytes_mut::<u32>(mutable_bytes).unwrap();
        *try_from_bytes_mut_ref = 0x0102_0304u32;
    }
    assert_eq!(mutable_source, 0x0102_0304u32);

    let unaligned_storage = [0xFF, 0x44, 0x33, 0x22, 0x11, 0xEE];
    let unaligned_read = bytemuck::pod_read_unaligned::<u32>(&unaligned_storage[1..5]);
    assert_eq!(unaligned_read, u32::from_ne_bytes([0x44, 0x33, 0x22, 0x11]));

    let try_unaligned_read =
        bytemuck::try_pod_read_unaligned::<u32>(&unaligned_storage[1..5]).unwrap();
    assert_eq!(try_unaligned_read, unaligned_read);

    assert!(bytemuck::try_from_bytes::<u32>(&source_bytes[..3]).is_err());
    assert!(bytemuck::try_pod_read_unaligned::<u32>(&unaligned_storage[..3]).is_err());
}

#[test]
fn zeroing_helpers_clear_single_values_and_slices() {
    let mut value = 0xFFFF_FFFFu32;
    bytemuck::write_zeroes(&mut value);
    assert_eq!(value, 0);

    let mut values = [1u32, 2, 3, 4];
    bytemuck::fill_zeroes(&mut values);
    assert_eq!(values, [0, 0, 0, 0]);

    let bytes = bytemuck::cast_slice::<u32, u8>(&values);
    assert!(bytes.iter().all(|byte| *byte == 0));
}

#[test]
fn pod_align_to_exposes_aligned_middle_region_without_losing_bytes() {
    let storage = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let (prefix, middle, suffix) = bytemuck::pod_align_to::<u8, u32>(&storage);

    assert_eq!(
        prefix.len() + middle.len() * std::mem::size_of::<u32>() + suffix.len(),
        storage.len()
    );

    let mut reconstructed = Vec::new();
    reconstructed.extend_from_slice(prefix);
    reconstructed.extend_from_slice(bytemuck::cast_slice::<u32, u8>(middle));
    reconstructed.extend_from_slice(suffix);
    assert_eq!(reconstructed, storage);

    let mut editable = [0u8; 16];
    let (prefix_len, middle_len, suffix_len) = {
        let (prefix, middle, suffix) = bytemuck::pod_align_to_mut::<u8, u32>(&mut editable);
        for (index, word) in middle.iter_mut().enumerate() {
            *word = 0x1111_0000u32 + index as u32;
        }
        (prefix.len(), middle.len(), suffix.len())
    };

    assert_eq!(
        prefix_len + middle_len * std::mem::size_of::<u32>() + suffix_len,
        editable.len()
    );

    let (_, middle_after, _) = bytemuck::pod_align_to::<u8, u32>(&editable);
    for (index, word) in middle_after.iter().enumerate() {
        assert_eq!(*word, 0x1111_0000u32 + index as u32);
    }
}