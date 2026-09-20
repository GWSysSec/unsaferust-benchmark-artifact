#![cfg(feature = "extern_crate_alloc")]

use bytemuck::allocation::{
  box_bytes_of, cast_arc, cast_box, cast_rc, cast_slice_arc, cast_slice_box,
  cast_slice_rc, cast_vec, from_box_bytes, pod_collect_to_vec, try_cast_arc,
  try_cast_box, try_cast_rc, try_cast_slice_arc, try_cast_slice_box,
  try_cast_slice_rc, try_cast_vec, try_from_box_bytes, try_zeroed_box,
  try_zeroed_slice_box, try_zeroed_vec, zeroed_arc, zeroed_arc_slice,
  zeroed_box, zeroed_rc, zeroed_rc_slice, zeroed_slice_box, zeroed_vec,
  BoxBytes,
};
use std::alloc::Layout;
use std::rc::Rc;
use std::sync::Arc;

#[test]
fn zeroed_allocations_produce_expected_values() {
  let boxed_u64: Box<u64> = zeroed_box();
  assert_eq!(*boxed_u64, 0);

  let fallible_boxed_array: Box<[u16; 4]> = try_zeroed_box().expect("zeroed box allocation");
  assert_eq!(*fallible_boxed_array, [0, 0, 0, 0]);

  let vec_u32: Vec<u32> = zeroed_vec(5);
  assert_eq!(vec_u32, vec![0, 0, 0, 0, 0]);

  let fallible_vec_u16: Vec<u16> = try_zeroed_vec(3).expect("zeroed vec allocation");
  assert_eq!(fallible_vec_u16, vec![0, 0, 0]);

  let empty_vec: Vec<u8> = zeroed_vec(0);
  assert!(empty_vec.is_empty());

  let boxed_slice: Box<[u32]> = zeroed_slice_box(4);
  assert_eq!(&*boxed_slice, &[0, 0, 0, 0]);

  let fallible_boxed_slice: Box<[u8]> =
    try_zeroed_slice_box(6).expect("zeroed slice box allocation");
  assert_eq!(&*fallible_boxed_slice, &[0, 0, 0, 0, 0, 0]);

  let arc_value: Arc<u64> = zeroed_arc();
  assert_eq!(*arc_value, 0);

  let arc_slice: Arc<[u16]> = zeroed_arc_slice(3);
  assert_eq!(&*arc_slice, &[0, 0, 0]);

  let rc_value: Rc<u32> = zeroed_rc();
  assert_eq!(*rc_value, 0);

  let rc_slice: Rc<[u8]> = zeroed_rc_slice(2);
  assert_eq!(&*rc_slice, &[0, 0]);
}

#[test]
fn owned_box_and_vec_casts_preserve_underlying_bytes() {
  let boxed_word = Box::new(0x0102_0304u32);
  let expected_bytes = 0x0102_0304u32.to_ne_bytes();

  let boxed_bytes: Box<[u8; 4]> = cast_box(boxed_word);
  assert_eq!(*boxed_bytes, expected_bytes);

  let round_tripped_box: Box<u32> =
    try_cast_box::<[u8; 4], u32>(boxed_bytes).expect("box casts back to u32");
  assert_eq!(*round_tripped_box, 0x0102_0304);

  let bad_box = Box::new([1u8, 2, 3]);
  assert!(try_cast_box::<[u8; 3], u32>(bad_box).is_err());

  let words = vec![0x1122u16, 0x3344, 0x5566];
  let expected_word_bytes: Vec<u8> =
    words.iter().flat_map(|v| u16::to_ne_bytes(*v)).collect();

  let bytes: Vec<u8> = cast_vec::<u16, u8>(words);
  assert_eq!(bytes, expected_word_bytes);

  let collected: Vec<u8> = pod_collect_to_vec::<u16, u8>(&[0xABCD, 0x1234]);
  let expected_collected: Vec<u8> = [0xABCDu16, 0x1234u16]
    .iter()
    .flat_map(|v| u16::to_ne_bytes(*v))
    .collect();
  assert_eq!(collected, expected_collected);

  let fallible_bytes = vec![0xAAu8, 0xBB, 0xCC];
  assert!(try_cast_vec::<u8, u32>(fallible_bytes).is_err());

  let cast_back_source = vec![0x10u16, 0x20, 0x30, 0x40];
  let cast_back_bytes: Vec<u8> = cast_vec::<u16, u8>(cast_back_source.clone());
  let cast_back_words: Vec<u16> =
    try_cast_vec::<u8, u16>(cast_back_bytes).expect("byte vec casts back to u16 vec");
  assert_eq!(cast_back_words, cast_back_source);
}

#[test]
fn boxed_slice_rc_and_arc_casts_adjust_lengths_and_preserve_data() {
  let boxed_words: Box<[u16]> = vec![0x0102u16, 0x0304, 0x0506].into_boxed_slice();
  let expected_bytes: Vec<u8> = boxed_words
    .iter()
    .flat_map(|v| u16::to_ne_bytes(*v))
    .collect();

  let boxed_bytes: Box<[u8]> = cast_slice_box::<u16, u8>(boxed_words);
  assert_eq!(&*boxed_bytes, expected_bytes.as_slice());

  let boxed_words_again: Box<[u16]> =
    try_cast_slice_box::<u8, u16>(boxed_bytes).expect("boxed byte slice casts back");
  assert_eq!(&*boxed_words_again, &[0x0102, 0x0304, 0x0506]);

  let bad_boxed_bytes: Box<[u8]> = vec![1u8, 2, 3].into_boxed_slice();
  assert!(try_cast_slice_box::<u8, u16>(bad_boxed_bytes).is_err());

  let rc_word: Rc<u32> = Rc::new(0xA1B2_C3D4);
  let rc_bytes: Rc<[u8; 4]> = cast_rc::<u32, [u8; 4]>(rc_word);
  assert_eq!(&*rc_bytes, &0xA1B2_C3D4u32.to_ne_bytes());

  let rc_word_again: Rc<u32> =
    try_cast_rc::<[u8; 4], u32>(rc_bytes).expect("rc bytes cast back to u32");
  assert_eq!(*rc_word_again, 0xA1B2_C3D4);

  let bad_rc: Rc<[u8; 3]> = Rc::new([1, 2, 3]);
  assert!(try_cast_rc::<[u8; 3], u32>(bad_rc).is_err());

  let arc_word: Arc<u32> = Arc::new(0x0102_0304);
  let arc_bytes: Arc<[u8; 4]> = cast_arc::<u32, [u8; 4]>(arc_word);
  assert_eq!(&*arc_bytes, &0x0102_0304u32.to_ne_bytes());

  let arc_word_again: Arc<u32> =
    try_cast_arc::<[u8; 4], u32>(arc_bytes).expect("arc bytes cast back to u32");
  assert_eq!(*arc_word_again, 0x0102_0304);

  let bad_arc: Arc<[u8; 3]> = Arc::new([9, 8, 7]);
  assert!(try_cast_arc::<[u8; 3], u32>(bad_arc).is_err());

  let rc_slice_words: Rc<[u16]> = Rc::from(vec![0x1112u16, 0x1314].into_boxed_slice());
  let rc_slice_expected: Vec<u8> = rc_slice_words
    .iter()
    .flat_map(|v| u16::to_ne_bytes(*v))
    .collect();
  let rc_slice_bytes: Rc<[u8]> = cast_slice_rc::<u16, u8>(rc_slice_words);
  assert_eq!(&*rc_slice_bytes, rc_slice_expected.as_slice());

  let rc_slice_words_again: Rc<[u16]> =
    try_cast_slice_rc::<u8, u16>(rc_slice_bytes).expect("rc slice bytes cast back");
  assert_eq!(&*rc_slice_words_again, &[0x1112, 0x1314]);

  let bad_rc_slice: Rc<[u8]> = Rc::from(vec![1u8, 2, 3].into_boxed_slice());
  assert!(try_cast_slice_rc::<u8, u16>(bad_rc_slice).is_err());

  let arc_slice_words: Arc<[u16]> = Arc::from(vec![0x2122u16, 0x2324].into_boxed_slice());
  let arc_slice_expected: Vec<u8> = arc_slice_words
    .iter()
    .flat_map(|v| u16::to_ne_bytes(*v))
    .collect();
  let arc_slice_bytes: Arc<[u8]> = cast_slice_arc::<u16, u8>(arc_slice_words);
  assert_eq!(&*arc_slice_bytes, arc_slice_expected.as_slice());

  let arc_slice_words_again: Arc<[u16]> =
    try_cast_slice_arc::<u8, u16>(arc_slice_bytes).expect("arc slice bytes cast back");
  assert_eq!(&*arc_slice_words_again, &[0x2122, 0x2324]);

  let bad_arc_slice: Arc<[u8]> = Arc::from(vec![1u8, 2, 3].into_boxed_slice());
  assert!(try_cast_slice_arc::<u8, u16>(bad_arc_slice).is_err());
}

#[test]
fn box_bytes_round_trips_and_exposes_layout_and_raw_parts() {
  let original = Box::new(0xDEAD_BEEFu32);
  let bytes = box_bytes_of(original);
  let expected_layout = Layout::new::<u32>();

  assert_eq!(bytes.layout().size(), expected_layout.size());
  assert_eq!(bytes.layout().align(), expected_layout.align());

  let restored: Box<u32> =
    try_from_box_bytes::<u32>(bytes).expect("BoxBytes created from u32 restores as u32");
  assert_eq!(*restored, 0xDEAD_BEEF);

  let byte_array = Box::new([10u8, 20, 30, 40]);
  let bytes = box_bytes_of(byte_array);
  let restored_array: Box<[u8; 4]> = from_box_bytes::<[u8; 4]>(bytes);
  assert_eq!(*restored_array, [10, 20, 30, 40]);

  let bad_bytes = box_bytes_of(Box::new([1u8, 2, 3]));
  assert!(try_from_box_bytes::<u32>(bad_bytes).is_err());

  let raw_source = Box::new([0x0102u16, 0x0304, 0x0506]);
  let raw_bytes = box_bytes_of(raw_source);
  let before_raw_layout = raw_bytes.layout();
  assert_eq!(before_raw_layout.size(), core::mem::size_of::<[u16; 3]>());
  assert_eq!(before_raw_layout.align(), core::mem::align_of::<[u16; 3]>());

  let (ptr, raw_layout) = raw_bytes.into_raw_parts();
  assert_eq!(raw_layout.size(), core::mem::size_of::<[u16; 3]>());
  assert_eq!(raw_layout.align(), core::mem::align_of::<[u16; 3]>());

  let rebuilt_bytes: BoxBytes = unsafe { BoxBytes::from_raw_parts(ptr, raw_layout) };
  assert_eq!(rebuilt_bytes.layout().size(), core::mem::size_of::<[u16; 3]>());

  let rebuilt: Box<[u16; 3]> = from_box_bytes::<[u16; 3]>(rebuilt_bytes);
  assert_eq!(*rebuilt, [0x0102, 0x0304, 0x0506]);
}