//! Collecting stats about heap memory objects.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::cell::Cell;
use crate::write_output;

thread_local! {
    /// Thread-local flag to skip tracking allocations made by the heap
    /// tracker's inner data structure.
    ///
    /// This prevents deadlock due to reentrant locking. Specifically, when
    /// allocating or deallocating a heap object from the application side, our
    /// heap tracker will also invoke allocation/deallocation to update its
    /// BTreeMap that maintains live object information. This will cause
    /// reentrant locking. To solve this issue, we use one TLS flag to indicate
    /// whether the locking is being held.
    static SKIP_TRACKING: Cell<bool> = Cell::new(false);
}

/// We classify heap objects into 14 groups by size:
/// - <= 1KB
/// - > 1KB && <= 2KB
/// - ...
/// - > 512 KB && <= 1 MB
/// - > 1 MB && <= 2 MB
/// - > 2 MB && <= 4 MB
/// - > 4 MB
/// 
/// NOTE: We should consider more fine-grained classification, as most unsafe
/// objects *should* be small.
const OBJ_SIZE_NUM: usize = 14;

/// Define the initial value for AtomicUsize.
/// 
/// This is needed to initialize an array of AtomicUsize for our size histograms.
/// The compiler disallows initializing such an array using [AtomicUsize::new(0); SIZE]
/// because AotmicUsize does not implement Copy.
const ATOMICUSIZE_INIT_0: AtomicUsize = AtomicUsize::new(0);

/// Core struct to collect heap data.
struct HeapTracker {
    // Total allocated heap objects in bytes.
    total_usage: AtomicUsize,
    // Total number of times allocating a heap object (including realloc)
    total_alloc: AtomicUsize,
    // Total number of times reallocating a heap object (including realloc)
    total_realloc: AtomicUsize,
    // Total number of times deallocating a heap object (including realloc)
    total_dealloc: AtomicUsize,
    // A BTreeMap containing ranges of all active heap objects
    live_objs: Mutex<BTreeMap<usize, usize>>,
    // Total number of unsafe heap objects.
    total_unsafe_objs: AtomicUsize,
    // Accumulated heap memory accessed by unsafe code
    unsafe_mem: AtomicUsize,
    // A set of live unsafe heap objects, represented by their starting addresses.
    live_unsafe_objs: Mutex<BTreeSet<usize>>,
    // Total number of heap memory access
    total_mem_insts: AtomicUsize,
    // Number of executed store instructions
    unsafe_load: AtomicUsize,
    // Number of executed store instructions
    unsafe_store: AtomicUsize,
    // Histogram of object sizes
    size_histogram: [AtomicUsize; OBJ_SIZE_NUM],
    // Histogram of unsafe object sizes
    unsafe_size_histogram: [AtomicUsize; OBJ_SIZE_NUM],
}

impl HeapTracker {
    pub const fn new() -> Self {
        Self {
            total_usage: AtomicUsize::new(0),
            total_alloc: AtomicUsize::new(0),
            total_realloc: AtomicUsize::new(0),
            total_dealloc: AtomicUsize::new(0),
            live_objs: Mutex::new(BTreeMap::new()),
            total_unsafe_objs: AtomicUsize::new(0),
            unsafe_mem: AtomicUsize::new(0),
            live_unsafe_objs: Mutex::new(BTreeSet::new()),
            total_mem_insts: ATOMICUSIZE_INIT_0,
            unsafe_load: AtomicUsize::new(0),
            unsafe_store: AtomicUsize::new(0),
            size_histogram: [ATOMICUSIZE_INIT_0; OBJ_SIZE_NUM],
            unsafe_size_histogram: [ATOMICUSIZE_INIT_0; OBJ_SIZE_NUM],
        }
    }

    /// Insert a newly allocated object into the map, including reallocated
    /// object with a new base address.
    fn insert_obj(&self, ptr: *mut u8, size: usize) {
        SKIP_TRACKING.with(|flag| {
            if flag.get() { return; }

            flag.set(true);
            self.live_objs.lock().unwrap().insert(ptr as usize, size);
            flag.set(false);
        });
    }

    /// Remove an object entry from the object map and also from the unsafe
    /// object set if the object is unsafe.
    fn remove_obj(&self, ptr: *mut u8) {
        SKIP_TRACKING.with(|flag| {
            if flag.get() { return; }

            flag.set(true);
            self.live_objs.lock().unwrap().remove(&(ptr as usize));
            self.live_unsafe_objs.lock().unwrap().remove(&(ptr as usize));
            flag.set(false);
        });
    }

    // A helper method to find if a heap object based on a given pointer.
    // If found, return the information about the object.
    fn find_heap_obj(&self, ptr: *const u8) -> Option<(usize, usize)> {
        SKIP_TRACKING.with(|flag| -> Option<(usize, usize)> {
            if flag.get() { return None; }
            flag.set(true);

            let addr = ptr as usize;
            let obj = self.live_objs.lock().unwrap()
                .range(..=addr)
                .last()
                .and_then(|(&base_addr, &size)|
                    (addr < base_addr + size).then_some((base_addr, size)));

            flag.set(false);
            return obj;
        })
    }

    /// Check whether a memory access is to a heap object.
    fn access_heap_obj(&self, ptr: *const u8) {
        if self.find_heap_obj(ptr).is_some() {
            self.total_mem_insts.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Check whether a target pointer points to an unsafe heap object. If so,
    ///  add this object into the unsafe object set if it has not been added.
    fn access_unsafe_heap_obj(&self, ptr: *const u8, is_load: bool) {
        if let Some((base_addr, size)) = self.find_heap_obj(ptr) {
            SKIP_TRACKING.with(|flag| {
                // We also need to SKIP_TRACKING-guard this function, as the
                // insertion of live_unsafe_objs may incur BTreeMaps's
                // internal heap allocation.
                if flag.get() { return; }
                flag.set(true);
                
                if self.live_unsafe_objs.lock().unwrap().insert(base_addr) {
                    // First time accessing this unsafe object.
                    self.unsafe_mem.fetch_add(size, Ordering::Relaxed);
                    self.total_unsafe_objs.fetch_add(1, Ordering::Relaxed);
                    self.classify_obj_by_size(size, true);
                }
                if is_load { self.unsafe_load.fetch_add(1, Ordering::Relaxed); }
                else { self.unsafe_store.fetch_add(1, Ordering::Relaxed); }

                flag.set(false);
            });
        }
    }

    /// Classify each heap allocation by its size.
    /// See the comment for OBJ_SIZE_NUM about the classification.
    fn classify_obj_by_size(&self, size: usize, is_unsafe: bool) {
        let size_histogram = if is_unsafe {&self.unsafe_size_histogram} else
                                          {&self.size_histogram };
        for i in 0..OBJ_SIZE_NUM - 1 {
            if size <= (1 << i) * (1 << 10) {
                size_histogram[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
        }

        size_histogram[OBJ_SIZE_NUM - 1].fetch_add(1, Ordering::Relaxed);
    }

    /// Convert an array of object size hisotgram to a string.
    fn size_hisogram_to_str(size_histogram: &[AtomicUsize; OBJ_SIZE_NUM]) -> String {
        size_histogram.iter()
                      .map(|v| v.load(Ordering::Relaxed).to_string())
                      .collect::<Vec<_>>().join("; ")
    }

    /// Print out heap usage stats.
    pub fn dump_stats(&self) {
        let heap_usage = self.total_usage.load(Ordering::Relaxed);
        let heap_alloc = self.total_alloc.load(Ordering::Relaxed);
        let heap_realloc = self.total_realloc.load(Ordering::Relaxed);
        let heap_dealloc = self.total_dealloc.load(Ordering::Relaxed);
        let unsafe_mem = self.unsafe_mem.load(Ordering::Relaxed);
        let unsafe_objs = self.total_unsafe_objs.load(Ordering::Relaxed);
        let total_mem_insts = self.total_mem_insts.load(Ordering::Relaxed);
        let unsafe_load = self.unsafe_load.load(Ordering::Relaxed);
        let unsafe_store = self.unsafe_store.load(Ordering::Relaxed);

        let size_histo = 
            SKIP_TRACKING.with(|flag| {
                // Skip tracking heap allocations invoked by the following code.
                // Without this, the code below somehow causes size_histo[0] to
                // be one greater total_alloc. It could be that (I'm not sure;
                // this is really weird!) some code was delayed executing until
                // the use of size_histo_str in the following format!(), and the
                // delayed code invokes one more small heap allocation which
                // increases historgram[0] by 1. However, this seems to be
                // implausible, because, why the heck would there be a delay?
                flag.set(true);

            Self::size_hisogram_to_str(&self.size_histogram)
        });
        let unsafe_size_histo = Self::size_hisogram_to_str(&self.unsafe_size_histogram);

        let output = format!(
            concat!(
                "\n===== Heap Usage Statistics =====\n",
                "Total heap usage: {} bytes\n",
                "Total heap allocations: {}\n",
                "Total heap re-allocations: {}\n",
                "Total heap deallocations: {}\n",
                "Unsafe heap memory: {}\n",
                "Unsafe heap objects: {}\n",
                "Unsafe memory instructions: {}\n",
                "Unsafe load: {}\n",
                "Unsafe store: {}\n",
                "Size histogram: {}\n",
                "Unsafe size histogram: {}\n",
            ),
            heap_usage, heap_alloc, heap_realloc, heap_dealloc, unsafe_mem, unsafe_objs,
            total_mem_insts, unsafe_load, unsafe_store, size_histo, unsafe_size_histo
        );

        // Write the output to a tmp file.
        // Note that we assume there is no output file in /tmp. This process will
        // otherwise append to an existing output file.
        //
        // TODO: Consider changing the output file name to heap_stat-process_name_or_pid.txt
        let _ = write_output(&output, "heap_stat.stat");

        // Only output to terminal for Debug build.
        if cfg!(debug_assertions) {
            dbg!("{}", &output);
        }
    }
}

unsafe impl GlobalAlloc for HeapTracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        let size = layout.size();

        // Add this new object's range to the map.
        if !ptr.is_null() && !SKIP_TRACKING.with(|flag| flag.get()) {
            self.total_usage.fetch_add(size, Ordering::Relaxed);
            self.total_alloc.fetch_add(1, Ordering::Relaxed);
            self.classify_obj_by_size(size, false);
            self.insert_obj(ptr, layout.size());
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !SKIP_TRACKING.with(|flag| flag.get()) {
            self.total_dealloc.fetch_add(1, Ordering::Relaxed);
            self.remove_obj(ptr);
        }

        System.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);

        if !ptr.is_null() && !SKIP_TRACKING.with(|flag| flag.get()) {
            self.total_usage.fetch_add(layout.size(), Ordering::Relaxed);
            self.total_alloc.fetch_add(1, Ordering::Relaxed);
            self.classify_obj_by_size(layout.size(), false);
            self.insert_obj(ptr, layout.size());
        }

        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if new_ptr.is_null() { return new_ptr; }

        if !SKIP_TRACKING.with(|flag| flag.get()) {
            if new_ptr != ptr {
                // Reallocating to a new address. Remove the old entry and record
                // the new entry.
                self.remove_obj(ptr);
                self.total_realloc.fetch_add(1, Ordering::Relaxed);
                self.classify_obj_by_size(new_size, false);
            }
            self.insert_obj(new_ptr, new_size);
        }

        // Update total heap usage if new_size differs than the old size.
        if new_size > layout.size() {
            self.total_usage.fetch_add(new_size - layout.size(), Ordering::Relaxed);
        } else {
            self.total_usage.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
        }

        new_ptr
    }
}

#[global_allocator]
static HEAP_TRACKER : HeapTracker = HeapTracker::new();

/// Process an unsafe memory access.
///
/// This function will be called (inserted by our LLVM pass) for each unsafe
/// memory access.
#[no_mangle]
#[inline(never)]
pub extern "C" fn dyn_unsafe_mem_access(ptr: *const u8, is_load: bool) {
    HEAP_TRACKER.access_unsafe_heap_obj(ptr, is_load);
}

/// Process a memory access instruction.
/// 
/// This function will be called for each memory instruction.
#[no_mangle]
#[inline(never)]
pub extern "C" fn dyn_mem_access(ptr: *const u8) {
    HEAP_TRACKER.access_heap_obj(ptr);
}

/// Dump heap usage stats at program termination time
#[ctor::dtor]
fn dump_stats() {
    HEAP_TRACKER.dump_stats();
}