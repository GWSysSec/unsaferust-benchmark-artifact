#![deny(warnings, rust_2018_idioms)]

use loom::alloc::{alloc, alloc_zeroed, dealloc};

use std::alloc::Layout;
use std::mem;
use std::ptr;

#[test]
fn alloc_write_read_and_dealloc_typed_values() {
    loom::model(|| {
        unsafe {
            let layout = Layout::array::<u32>(4).expect("valid u32 array layout");
            let ptr = alloc(layout);
            assert!(!ptr.is_null());

            let values = ptr.cast::<u32>();
            for index in 0..4 {
                ptr::write(values.add(index), (index as u32 + 1) * 10);
            }

            assert_eq!(ptr::read(values.add(0)), 10);
            assert_eq!(ptr::read(values.add(1)), 20);
            assert_eq!(ptr::read(values.add(2)), 30);
            assert_eq!(ptr::read(values.add(3)), 40);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn alloc_zeroed_provides_zero_initialized_bytes_before_mutation() {
    loom::model(|| {
        unsafe {
            let layout = Layout::from_size_align(16, 8).expect("valid byte buffer layout");
            let ptr = alloc_zeroed(layout);
            assert!(!ptr.is_null());

            for offset in 0..16 {
                assert_eq!(*ptr.add(offset), 0);
            }

            *ptr.add(0) = 0xAB;
            *ptr.add(7) = 0xCD;
            *ptr.add(15) = 0xEF;

            assert_eq!(*ptr.add(0), 0xAB);
            assert_eq!(*ptr.add(7), 0xCD);
            assert_eq!(*ptr.add(15), 0xEF);
            assert_eq!(*ptr.add(8), 0);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn multiple_allocations_are_independent_and_can_be_deallocated_in_reverse_order() {
    loom::model(|| {
        unsafe {
            let left_layout = Layout::new::<u64>();
            let right_layout = Layout::new::<u64>();

            let left = alloc(left_layout);
            let right = alloc_zeroed(right_layout);

            assert!(!left.is_null());
            assert!(!right.is_null());
            assert_ne!(left, right);

            let left_value = left.cast::<u64>();
            let right_value = right.cast::<u64>();

            assert_eq!(ptr::read(right_value), 0);

            ptr::write(left_value, 0x1111_2222_3333_4444);
            ptr::write(right_value, 0xAAAA_BBBB_CCCC_DDDD);

            assert_eq!(ptr::read(left_value), 0x1111_2222_3333_4444);
            assert_eq!(ptr::read(right_value), 0xAAAA_BBBB_CCCC_DDDD);

            dealloc(right, right_layout);
            dealloc(left, left_layout);
        }
    });
}

#[test]
fn allocation_with_custom_alignment_supports_aligned_access() {
    loom::model(|| {
        unsafe {
            let layout = Layout::from_size_align(32, 16).expect("valid aligned layout");
            let ptr = alloc(layout);
            assert!(!ptr.is_null());
            assert_eq!((ptr as usize) % 16, 0);

            for offset in 0..32 {
                *ptr.add(offset) = offset as u8;
            }

            assert_eq!(*ptr.add(0), 0);
            assert_eq!(*ptr.add(15), 15);
            assert_eq!(*ptr.add(16), 16);
            assert_eq!(*ptr.add(31), 31);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn realloc_like_copy_between_loom_allocations_preserves_contents() {
    loom::model(|| {
        unsafe {
            let small_layout = Layout::array::<u8>(4).expect("valid small layout");
            let large_layout = Layout::array::<u8>(8).expect("valid large layout");

            let small = alloc(small_layout);
            let large = alloc_zeroed(large_layout);

            assert!(!small.is_null());
            assert!(!large.is_null());

            for index in 0..4 {
                *small.add(index) = (index as u8) + 1;
            }

            ptr::copy_nonoverlapping(small, large, 4);

            assert_eq!(*large.add(0), 1);
            assert_eq!(*large.add(1), 2);
            assert_eq!(*large.add(2), 3);
            assert_eq!(*large.add(3), 4);
            assert_eq!(*large.add(4), 0);
            assert_eq!(*large.add(7), 0);

            dealloc(small, small_layout);
            dealloc(large, large_layout);
        }
    });
}

#[test]
fn manual_allocation_can_store_and_drop_non_copy_value() {
    loom::model(|| {
        unsafe {
            let layout = Layout::new::<String>();
            assert_eq!(layout.size(), mem::size_of::<String>());

            let ptr = alloc(layout);
            assert!(!ptr.is_null());

            let string_ptr = ptr.cast::<String>();
            ptr::write(string_ptr, String::from("loom allocation"));

            assert_eq!((&*string_ptr).len(), 15);
            assert_eq!(&*string_ptr, "loom allocation");

            ptr::drop_in_place(string_ptr);
            dealloc(ptr, layout);
        }
    });
}