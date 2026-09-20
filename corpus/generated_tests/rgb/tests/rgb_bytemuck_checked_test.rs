use rgb::bytemuck::checked;

#[test]
fn checked_casts_support_pixel_channel_like_bool_workflow() {
    let opaque: bool = checked::cast::<u8, bool>(1);
    assert!(opaque);

    let transparent: &bool = checked::cast_ref::<u8, bool>(&0u8);
    assert!(!*transparent);

    let mask_bytes = [1u8, 0, 1, 1, 0, 0, 1, 0];
    let mask: &[bool] = checked::cast_slice::<u8, bool>(&mask_bytes);
    assert_eq!(mask.len(), mask_bytes.len());
    assert_eq!(mask, &[true, false, true, true, false, false, true, false]);

    let mut flag_byte = 0u8;
    {
        let flag: &mut bool = checked::cast_mut::<u8, bool>(&mut flag_byte);
        assert!(!*flag);
        *flag = true;
    }
    assert_eq!(flag_byte, 1);

    let mut editable_mask_bytes = [0u8, 1, 0, 1];
    {
        let editable_mask: &mut [bool] =
            checked::cast_slice_mut::<u8, bool>(&mut editable_mask_bytes);
        assert_eq!(editable_mask, &[false, true, false, true]);
        editable_mask[0] = true;
        editable_mask[1] = false;
        editable_mask[3] = false;
    }
    assert_eq!(editable_mask_bytes, [1, 0, 0, 0]);
}

#[test]
fn checked_byte_views_round_trip_aligned_numeric_storage() {
    let mut word = 0x1122_3344u32;

    {
        let bytes: &[u8; 4] = checked::cast_ref::<u32, [u8; 4]>(&word);
        let from_bytes_word: &u32 = checked::from_bytes::<u32>(&bytes[..]);
        assert_eq!(*from_bytes_word, 0x1122_3344);

        let copied: u32 = checked::pod_read_unaligned::<u32>(&bytes[..]);
        assert_eq!(copied, word);
    }

    {
        let bytes: &mut [u8; 4] = checked::cast_mut::<u32, [u8; 4]>(&mut word);
        let mutable_word: &mut u32 = checked::from_bytes_mut::<u32>(&mut bytes[..]);
        *mutable_word = 0xAABB_CCDD;
    }

    assert_eq!(word, 0xAABB_CCDD);

    let bytes_again: &[u8; 4] = checked::cast_ref::<u32, [u8; 4]>(&word);
    let reread: u32 = checked::pod_read_unaligned::<u32>(&bytes_again[..]);
    assert_eq!(reread, 0xAABB_CCDD);
}

#[test]
fn try_checked_casts_report_invalid_patterns_and_size_errors() {
    assert_eq!(checked::try_cast::<u8, bool>(0).unwrap(), false);
    assert_eq!(checked::try_cast::<u8, bool>(1).unwrap(), true);
    assert!(checked::try_cast::<u8, bool>(2).is_err());
    assert!(checked::try_cast::<u16, bool>(1).is_err());

    let valid = 1u8;
    let valid_ref: &bool = checked::try_cast_ref::<u8, bool>(&valid).unwrap();
    assert!(*valid_ref);

    let invalid = 3u8;
    assert!(checked::try_cast_ref::<u8, bool>(&invalid).is_err());

    let valid_slice_bytes = [0u8, 1, 1, 0];
    let valid_slice: &[bool] = checked::try_cast_slice::<u8, bool>(&valid_slice_bytes).unwrap();
    assert_eq!(valid_slice, &[false, true, true, false]);

    let invalid_slice_bytes = [0u8, 1, 2, 0];
    assert!(checked::try_cast_slice::<u8, bool>(&invalid_slice_bytes).is_err());

    let uneven_words = [0x1234u16, 0x5678, 0x9ABCu16];
    assert!(checked::try_cast_slice::<u16, u32>(&uneven_words).is_err());
}

#[test]
fn try_checked_mutable_casts_allow_in_place_updates_only_for_valid_patterns() {
    let mut byte = 1u8;
    {
        let flag: &mut bool = checked::try_cast_mut::<u8, bool>(&mut byte).unwrap();
        assert!(*flag);
        *flag = false;
    }
    assert_eq!(byte, 0);

    let mut invalid_byte = 7u8;
    assert!(checked::try_cast_mut::<u8, bool>(&mut invalid_byte).is_err());
    assert_eq!(invalid_byte, 7);

    let mut mask_bytes = [1u8, 0, 1, 0];
    {
        let mask: &mut [bool] =
            checked::try_cast_slice_mut::<u8, bool>(&mut mask_bytes).unwrap();
        assert_eq!(mask, &[true, false, true, false]);
        mask[1] = true;
        mask[2] = false;
    }
    assert_eq!(mask_bytes, [1, 1, 0, 0]);

    let mut invalid_mask_bytes = [0u8, 2, 1];
    assert!(checked::try_cast_slice_mut::<u8, bool>(&mut invalid_mask_bytes).is_err());
    assert_eq!(invalid_mask_bytes, [0, 2, 1]);
}

#[test]
fn try_from_bytes_and_try_unaligned_reads_validate_length_and_bit_patterns() {
    let valid_bool_bytes = [1u8];
    let true_ref: &bool = checked::try_from_bytes::<bool>(&valid_bool_bytes).unwrap();
    assert!(*true_ref);

    let invalid_bool_bytes = [2u8];
    assert!(checked::try_from_bytes::<bool>(&invalid_bool_bytes).is_err());

    let wrong_len = [0u8, 1];
    assert!(checked::try_from_bytes::<bool>(&wrong_len).is_err());

    let read_false: bool = checked::try_pod_read_unaligned::<bool>(&[0u8]).unwrap();
    assert!(!read_false);
    assert!(checked::try_pod_read_unaligned::<bool>(&[9u8]).is_err());
    assert!(checked::try_pod_read_unaligned::<u32>(&[1u8, 2, 3]).is_err());

    let mut editable_bool_byte = [0u8];
    {
        let value: &mut bool = checked::try_from_bytes_mut::<bool>(&mut editable_bool_byte).unwrap();
        assert!(!*value);
        *value = true;
    }
    assert_eq!(editable_bool_byte, [1]);

    let mut invalid_editable_bool_byte = [4u8];
    assert!(checked::try_from_bytes_mut::<bool>(&mut invalid_editable_bool_byte).is_err());
    assert_eq!(invalid_editable_bool_byte, [4]);
}