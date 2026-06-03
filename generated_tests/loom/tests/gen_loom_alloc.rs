use loom::alloc::{alloc, alloc_zeroed, dealloc};
use loom::sync::atomic::AtomicUsize;
use loom::sync::Arc;
use loom::thread;
use std::alloc::Layout;
use std::sync::atomic::Ordering::{Relaxed, Release, Acquire};

#[test]
fn alloc_basic_single_byte() {
    loom::model(|| {
        unsafe {
            let layout = Layout::from_size_align(1, 1).unwrap();
            let ptr = alloc(layout);
            assert!(!ptr.is_null());


            *ptr = 42u8;
            assert_eq!(*ptr, 42u8);


            *ptr = 255u8;
            assert_eq!(*ptr, 255u8);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn alloc_multiple_bytes_and_write_pattern() {
    loom::model(|| {
        unsafe {
            let size = 64;
            let layout = Layout::from_size_align(size, 8).unwrap();
            let ptr = alloc(layout);
            assert!(!ptr.is_null());


            for i in 0..size {
                *ptr.add(i) = (i & 0xFF) as u8;
            }


            assert_eq!(*ptr.add(0), 0u8);
            assert_eq!(*ptr.add(1), 1u8);
            assert_eq!(*ptr.add(10), 10u8);
            assert_eq!(*ptr.add(63), 63u8);
            assert_eq!(*ptr.add(32), 32u8);
            assert_eq!(*ptr.add(55), 55u8);
            assert_eq!(*ptr.add(7), 7u8);
            assert_eq!(*ptr.add(15), 15u8);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn alloc_zeroed_basic() {
    loom::model(|| {
        unsafe {
            let size = 128;
            let layout = Layout::from_size_align(size, 8).unwrap();
            let ptr = alloc_zeroed(layout);
            assert!(!ptr.is_null());


            assert_eq!(*ptr.add(0), 0u8);
            assert_eq!(*ptr.add(1), 0u8);
            assert_eq!(*ptr.add(63), 0u8);
            assert_eq!(*ptr.add(64), 0u8);
            assert_eq!(*ptr.add(100), 0u8);
            assert_eq!(*ptr.add(127), 0u8);
            assert_eq!(*ptr.add(50), 0u8);
            assert_eq!(*ptr.add(99), 0u8);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn alloc_zeroed_then_write_and_verify() {
    loom::model(|| {
        unsafe {
            let size = 32;
            let layout = Layout::from_size_align(size, 4).unwrap();
            let ptr = alloc_zeroed(layout);
            assert!(!ptr.is_null());


            assert_eq!(*ptr.add(0), 0u8);
            assert_eq!(*ptr.add(15), 0u8);
            assert_eq!(*ptr.add(31), 0u8);


            *ptr.add(0) = 0xDE;
            *ptr.add(1) = 0xAD;
            *ptr.add(2) = 0xBE;
            *ptr.add(3) = 0xEF;
            *ptr.add(31) = 0xFF;


            assert_eq!(*ptr.add(0), 0xDE);
            assert_eq!(*ptr.add(1), 0xAD);
            assert_eq!(*ptr.add(2), 0xBE);
            assert_eq!(*ptr.add(3), 0xEF);
            assert_eq!(*ptr.add(31), 0xFF);


            assert_eq!(*ptr.add(4), 0u8);
            assert_eq!(*ptr.add(16), 0u8);
            assert_eq!(*ptr.add(30), 0u8);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn alloc_dealloc_multiple_allocations() {
    loom::model(|| {
        unsafe {
            let layout1 = Layout::from_size_align(16, 4).unwrap();
            let layout2 = Layout::from_size_align(32, 8).unwrap();
            let layout3 = Layout::from_size_align(8, 1).unwrap();

            let ptr1 = alloc(layout1);
            let ptr2 = alloc(layout2);
            let ptr3 = alloc_zeroed(layout3);

            assert!(!ptr1.is_null());
            assert!(!ptr2.is_null());
            assert!(!ptr3.is_null());


            assert_ne!(ptr1, ptr2);
            assert_ne!(ptr2, ptr3);
            assert_ne!(ptr1, ptr3);


            *ptr1 = 0xAA;
            *ptr2 = 0xBB;


            assert_eq!(*ptr3.add(0), 0u8);
            assert_eq!(*ptr3.add(7), 0u8);


            assert_eq!(*ptr1, 0xAA);
            assert_eq!(*ptr2, 0xBB);

            dealloc(ptr3, layout3);
            dealloc(ptr2, layout2);
            dealloc(ptr1, layout1);
        }
    });
}

#[test]
fn alloc_with_various_alignments() {
    loom::model(|| {
        unsafe {
            let layout_align1 = Layout::from_size_align(16, 1).unwrap();
            let layout_align4 = Layout::from_size_align(16, 4).unwrap();
            let layout_align8 = Layout::from_size_align(16, 8).unwrap();
            let layout_align16 = Layout::from_size_align(16, 16).unwrap();

            let ptr1 = alloc(layout_align1);
            let ptr4 = alloc(layout_align4);
            let ptr8 = alloc(layout_align8);
            let ptr16 = alloc(layout_align16);

            assert!(!ptr1.is_null());
            assert!(!ptr4.is_null());
            assert!(!ptr8.is_null());
            assert!(!ptr16.is_null());


            assert_eq!((ptr4 as usize) % 4, 0);
            assert_eq!((ptr8 as usize) % 8, 0);
            assert_eq!((ptr16 as usize) % 16, 0);


            *ptr1 = 1;
            *ptr4 = 4;
            *ptr8 = 8;
            *ptr16 = 16;

            assert_eq!(*ptr1, 1);
            assert_eq!(*ptr4, 4);
            assert_eq!(*ptr8, 8);
            assert_eq!(*ptr16, 16);

            dealloc(ptr16, layout_align16);
            dealloc(ptr8, layout_align8);
            dealloc(ptr4, layout_align4);
            dealloc(ptr1, layout_align1);
        }
    });
}

#[test]
fn alloc_concurrent_threads_use_separate_allocations() {
    loom::model(|| {
        let counter = Arc::new(AtomicUsize::new(0));

        let c1 = counter.clone();
        let t1 = thread::spawn(move || {
            unsafe {
                let layout = Layout::from_size_align(16, 4).unwrap();
                let ptr = alloc_zeroed(layout);
                assert!(!ptr.is_null());
                assert_eq!(*ptr.add(0), 0u8);
                assert_eq!(*ptr.add(15), 0u8);
                *ptr = 0xAB;
                assert_eq!(*ptr, 0xAB);
                dealloc(ptr, layout);
            }
            c1.fetch_add(1, Release);
        });

        let c2 = counter.clone();
        let t2 = thread::spawn(move || {
            unsafe {
                let layout = Layout::from_size_align(8, 2).unwrap();
                let ptr = alloc(layout);
                assert!(!ptr.is_null());
                *ptr = 0xCD;
                assert_eq!(*ptr, 0xCD);
                dealloc(ptr, layout);
            }
            c2.fetch_add(1, Release);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        assert_eq!(counter.load(Acquire), 2);
    });
}

#[test]
fn alloc_zeroed_large_block() {
    loom::model(|| {
        unsafe {
            let size = 4096;
            let layout = Layout::from_size_align(size, 16).unwrap();
            let ptr = alloc_zeroed(layout);
            assert!(!ptr.is_null());


            assert_eq!(*ptr.add(0), 0u8);
            assert_eq!(*ptr.add(1000), 0u8);
            assert_eq!(*ptr.add(2048), 0u8);
            assert_eq!(*ptr.add(4095), 0u8);
            assert_eq!(*ptr.add(512), 0u8);
            assert_eq!(*ptr.add(3000), 0u8);


            *ptr.add(0) = 0x11;
            *ptr.add(4095) = 0x22;

            assert_eq!(*ptr.add(0), 0x11);
            assert_eq!(*ptr.add(4095), 0x22);

            assert_eq!(*ptr.add(2048), 0u8);

            dealloc(ptr, layout);
        }
    });
}

#[test]
fn alloc_reuse_after_dealloc() {
    loom::model(|| {
        unsafe {
            let layout = Layout::from_size_align(64, 8).unwrap();


            let ptr1 = alloc(layout);
            assert!(!ptr1.is_null());
            *ptr1 = 0x99;
            assert_eq!(*ptr1, 0x99);
            dealloc(ptr1, layout);


            let ptr2 = alloc_zeroed(layout);
            assert!(!ptr2.is_null());

            assert_eq!(*ptr2.add(0), 0u8);
            assert_eq!(*ptr2.add(32), 0u8);
            assert_eq!(*ptr2.add(63), 0u8);


            let ptr3 = alloc(layout);
            assert!(!ptr3.is_null());
            *ptr3 = 0x77;
            assert_eq!(*ptr3, 0x77);

            dealloc(ptr3, layout);
            dealloc(ptr2, layout);
        }
    });
}

#[test]
fn alloc_store_typed_data_via_raw_pointer() {
    loom::model(|| {
        unsafe {
            let layout = Layout::from_size_align(
                std::mem::size_of::<u64>() * 4,
                std::mem::align_of::<u64>(),
            )
            .unwrap();

            let ptr = alloc_zeroed(layout);
            assert!(!ptr.is_null());
            assert_eq!((ptr as usize) % std::mem::align_of::<u64>(), 0);

            let typed_ptr = ptr as *mut u64;


            assert_eq!(*typed_ptr.add(0), 0u64);
            assert_eq!(*typed_ptr.add(1), 0u64);
            assert_eq!(*typed_ptr.add(2), 0u64);
            assert_eq!(*typed_ptr.add(3), 0u64);


            *typed_ptr.add(0) = 0xDEAD_BEEF_CAFE_BABE;
            *typed_ptr.add(1) = u64::MAX;
            *typed_ptr.add(2) = 42;
            *typed_ptr.add(3) = 0;


            assert_eq!(*typed_ptr.add(0), 0xDEAD_BEEF_CAFE_BABE);
            assert_eq!(*typed_ptr.add(1), u64::MAX);
            assert_eq!(*typed_ptr.add(2), 42);
            assert_eq!(*typed_ptr.add(3), 0);

            dealloc(ptr, layout);
        }
    });
}