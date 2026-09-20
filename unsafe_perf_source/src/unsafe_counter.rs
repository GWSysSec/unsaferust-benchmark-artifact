//! Runtime library for UnsafeCount LLVM passes
//!
//! Lock-free, zero-allocation runtime supporting the two-pass system:
//! - UnsafeFunctionTrackerPass (module pass): tracks function calls and
//!   function-level static unsafe classification
//! - UnsafeInstCounterPass (function pass): counts unsafe instructions and
//!   marks functions whose unsafe instructions actually executed

use std::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};
use std::mem::MaybeUninit;
use crate::write_stat_json;

/// Maximum number of functions we can track
const MAX_FUNCTIONS: usize = 1_048_576;

/// Function metadata from compile-time analysis.
///
/// `is_unsafe` is 1 iff the function has both (1) at least one validated
/// SESE region and (2) at least one instruction carrying `!unsafe_inst`
/// metadata inside one of those regions — i.e., unsafe code that survived
/// optimization.  Every tracked function still enters the table regardless,
/// so `metadata_count` is the total function denominator.
///
/// Layout must match the LLVM side in UnsafeFunctionTracker.cpp:
///   struct { u32 id; u8 is_unsafe; u8 _padding[3]; }
#[repr(C)]
#[derive(Copy, Clone)]
struct FunctionMetadata {
    id: u32,
    is_unsafe: u8,
    _padding: [u8; 3],
}

/// Lock-free bitset for tracking unique functions
struct AtomicBitset {
    words: [CachePadded<AtomicU64>; (MAX_FUNCTIONS + 63) / 64],
}

/// Cache-line padded atomic to prevent false sharing
#[repr(align(64))]
struct CachePadded<T> {
    value: T,
}

impl AtomicBitset {
    const fn new() -> Self {
        const ZERO: CachePadded<AtomicU64> = CachePadded {
            value: AtomicU64::new(0),
        };
        Self {
            words: [ZERO; (MAX_FUNCTIONS + 63) / 64],
        }
    }
    
    #[inline]
    fn set(&self, index: usize) {
        let word_idx = index / 64;
        let bit_idx = index % 64;
        self.words[word_idx].value.fetch_or(1u64 << bit_idx, Ordering::Relaxed);
    }
    
    #[inline]
    fn is_set(&self, index: usize) -> bool {
        let word_idx = index / 64;
        let bit_idx = index % 64;
        (self.words[word_idx].value.load(Ordering::Relaxed) & (1u64 << bit_idx)) != 0
    }
}

/// Main tracker structure - all fixed-size, no allocations
struct UnsafeTracker {
    // ===== Function Tracking (from UnsafeFunctionTrackerPass) =====
    
    // Function metadata from compile time (read-only after init)
    metadata: [MaybeUninit<FunctionMetadata>; MAX_FUNCTIONS],
    metadata_count: AtomicU32,
    
    // Per-function call counts
    function_calls: [CachePadded<AtomicU64>; MAX_FUNCTIONS],
    
    // Bitset for tracking which functions were executed
    functions_seen: AtomicBitset,

    // Bitset for tracking which functions actually executed unsafe instructions
    unsafe_functions_seen_dynamic: AtomicBitset,
    
    // ===== Instruction Counting (from UnsafeInstCounterPass) =====
    
    // Global instruction counters
    total_instructions: AtomicU64,
    total_unsafe_instructions: AtomicU64,
    
    // Unsafe instruction type counters (9 categories)
    unsafe_loads: AtomicU64,
    unsafe_stores: AtomicU64,
    unsafe_calls_direct: AtomicU64,
    unsafe_calls_indirect: AtomicU64,
    unsafe_calls_intrinsic: AtomicU64,
    unsafe_casts: AtomicU64,
    unsafe_geps: AtomicU64,
    unsafe_atomics: AtomicU64,
    unsafe_others: AtomicU64,
    
    // ===== Control =====
    
    // Ensure stats are written only once
    stats_written: AtomicBool,
    
    // Track if metadata has been initialized
    metadata_initialized: AtomicBool,
}

impl UnsafeTracker {
    const fn new() -> Self {
        const ZERO_PADDED: CachePadded<AtomicU64> = CachePadded {
            value: AtomicU64::new(0),
        };
        const UNINIT: MaybeUninit<FunctionMetadata> = MaybeUninit::uninit();
        
        Self {
            // Function tracking
            metadata: [UNINIT; MAX_FUNCTIONS],
            metadata_count: AtomicU32::new(0),
            function_calls: [ZERO_PADDED; MAX_FUNCTIONS],
            functions_seen: AtomicBitset::new(),
            unsafe_functions_seen_dynamic: AtomicBitset::new(),
            
            // Instruction counting
            total_instructions: AtomicU64::new(0),
            total_unsafe_instructions: AtomicU64::new(0),
            unsafe_loads: AtomicU64::new(0),
            unsafe_stores: AtomicU64::new(0),
            unsafe_calls_direct: AtomicU64::new(0),
            unsafe_calls_indirect: AtomicU64::new(0),
            unsafe_calls_intrinsic: AtomicU64::new(0),
            unsafe_casts: AtomicU64::new(0),
            unsafe_geps: AtomicU64::new(0),
            unsafe_atomics: AtomicU64::new(0),
            unsafe_others: AtomicU64::new(0),
            
            // Control
            stats_written: AtomicBool::new(false),
            metadata_initialized: AtomicBool::new(false),
        }
    }
    
    // ===== Functions called by UnsafeFunctionTrackerPass =====
    
    /// Append metadata from one CGU's compile-time table.
    /// Returns the base offset — the caller must add this to local function IDs
    /// so that IDs are globally unique across CGUs.
    /// Called by each CGU's module constructor.
    unsafe fn init_metadata(&self, metadata_ptr: *const u8, count: u32) -> u32 {
        // Atomically reserve a slot range for this CGU
        let base = self.metadata_count.fetch_add(count, Ordering::AcqRel);

        if (base + count) as usize > MAX_FUNCTIONS {
            eprintln!(
                "Warning: Function count overflow ({} + {} > {})",
                base, count, MAX_FUNCTIONS
            );
            // Roll back
            self.metadata_count.fetch_sub(count, Ordering::Relaxed);
            return base;
        }

        let metadata_slice = std::slice::from_raw_parts(
            metadata_ptr as *const FunctionMetadata,
            count as usize,
        );

        let meta_ptr = self.metadata.as_ptr() as *mut MaybeUninit<FunctionMetadata>;

        for (i, meta) in metadata_slice.iter().enumerate() {
            // Remap the local ID to the global ID
            let mut remapped = *meta;
            remapped.id = base + i as u32;
            (*meta_ptr.add(base as usize + i)).write(remapped);
        }

        self.metadata_initialized.store(true, Ordering::Release);
        base
    }
    
    /// Record a function call - called at each function entry
    #[inline(always)]
    fn record_function(&self, func_id: u32) {
        if func_id as usize >= MAX_FUNCTIONS {
            return;
        }
        
        // Two atomic operations: increment counter and set bit
        self.function_calls[func_id as usize].value.fetch_add(1, Ordering::Relaxed);
        self.functions_seen.set(func_id as usize);
    }
    
    // ===== Functions called by UnsafeInstCounterPass =====
    
    /// Record basic block statistics - called per basic block
    #[inline(always)]
    fn record_block(&self,
        func_id: u32,
        total: u32,
        unsafe_total: u32,
        unsafe_load: u16,
        unsafe_store: u16,
        unsafe_call_direct: u16,
        unsafe_call_indirect: u16,
        unsafe_call_intrinsic: u16,
        unsafe_cast: u16,
        unsafe_gep: u16,
        unsafe_atomic: u16,
        unsafe_other: u16
    ) {
        self.total_instructions.fetch_add(total as u64, Ordering::Relaxed);

        if unsafe_total == 0 {
            return;
        }

        if (func_id as usize) < MAX_FUNCTIONS {
            self.unsafe_functions_seen_dynamic.set(func_id as usize);
        }

        self.total_unsafe_instructions.fetch_add(unsafe_total as u64, Ordering::Relaxed);

        if unsafe_load > 0 {
            self.unsafe_loads.fetch_add(unsafe_load as u64, Ordering::Relaxed);
        }
        if unsafe_store > 0 {
            self.unsafe_stores.fetch_add(unsafe_store as u64, Ordering::Relaxed);
        }
        if unsafe_call_direct > 0 {
            self.unsafe_calls_direct.fetch_add(unsafe_call_direct as u64, Ordering::Relaxed);
        }
        if unsafe_call_indirect > 0 {
            self.unsafe_calls_indirect.fetch_add(unsafe_call_indirect as u64, Ordering::Relaxed);
        }
        if unsafe_call_intrinsic > 0 {
            self.unsafe_calls_intrinsic.fetch_add(unsafe_call_intrinsic as u64, Ordering::Relaxed);
        }
        if unsafe_cast > 0 {
            self.unsafe_casts.fetch_add(unsafe_cast as u64, Ordering::Relaxed);
        }
        if unsafe_gep > 0 {
            self.unsafe_geps.fetch_add(unsafe_gep as u64, Ordering::Relaxed);
        }
        if unsafe_atomic > 0 {
            self.unsafe_atomics.fetch_add(unsafe_atomic as u64, Ordering::Relaxed);
        }
        if unsafe_other > 0 {
            self.unsafe_others.fetch_add(unsafe_other as u64, Ordering::Relaxed);
        }
    }
    
    // ===== Statistics Output =====
    
    /// Calculate and dump statistics
    fn dump_stats(&self) {
        // Ensure single execution
        if self.stats_written.swap(true, Ordering::AcqRel) {
            return;
        }
        
        // Check if metadata was initialized
        if !self.metadata_initialized.load(Ordering::Acquire) {
            return;
        }
        
        let metadata_count = self.metadata_count.load(Ordering::Acquire) as usize;
        if metadata_count == 0 {
            return;
        }
        
        // Load instruction statistics
        let total_insts = self.total_instructions.load(Ordering::Relaxed);
        let unsafe_insts = self.total_unsafe_instructions.load(Ordering::Relaxed);
        let unsafe_loads = self.unsafe_loads.load(Ordering::Relaxed);
        let unsafe_stores = self.unsafe_stores.load(Ordering::Relaxed);
        let unsafe_calls_direct = self.unsafe_calls_direct.load(Ordering::Relaxed);
        let unsafe_calls_indirect = self.unsafe_calls_indirect.load(Ordering::Relaxed);
        let unsafe_calls_intrinsic = self.unsafe_calls_intrinsic.load(Ordering::Relaxed);
        let unsafe_casts = self.unsafe_casts.load(Ordering::Relaxed);
        let unsafe_geps = self.unsafe_geps.load(Ordering::Relaxed);
        let unsafe_atomics = self.unsafe_atomics.load(Ordering::Relaxed);
        let unsafe_others = self.unsafe_others.load(Ordering::Relaxed);
        
        // Calculate function statistics
        let mut functions_executed = 0u32;
        let mut unsafe_functions_defined = 0u32;
        let mut unsafe_functions_executed = 0u32;
        let mut unsafe_functions_with_executed_insts = 0u32;
        let mut total_function_calls = 0u64;
        let mut unsafe_function_calls = 0u64;

        for i in 0..metadata_count {
            let meta = unsafe { self.metadata[i].assume_init() };
            let is_unsafe = meta.is_unsafe != 0;
            let executed_unsafe_insts = self.unsafe_functions_seen_dynamic.is_set(i);

            if is_unsafe {
                unsafe_functions_defined += 1;
            }

            if executed_unsafe_insts {
                unsafe_functions_with_executed_insts += 1;
            }

            if self.functions_seen.is_set(i) {
                functions_executed += 1;

                if is_unsafe {
                    unsafe_functions_executed += 1;
                }

                // Get call count
                let calls = self.function_calls[i].value.load(Ordering::Relaxed);
                total_function_calls += calls;

                if is_unsafe {
                    unsafe_function_calls += calls;
                }
            }
        }
        
        let stats_json = format!(
            concat!(
                "{{",
                "\"total_instructions\":{},",
                "\"unsafe_instructions\":{},",
                "\"unsafe_loads\":{},",
                "\"unsafe_stores\":{},",
                "\"unsafe_calls_direct\":{},",
                "\"unsafe_calls_indirect\":{},",
                "\"unsafe_calls_intrinsic\":{},",
                "\"unsafe_casts\":{},",
                "\"unsafe_geps\":{},",
                "\"unsafe_atomics\":{},",
                "\"unsafe_others\":{},",
                "\"total_functions_defined\":{},",
                "\"unsafe_functions_defined\":{},",
                "\"total_functions_executed\":{},",
                "\"unsafe_functions_executed\":{},",
                "\"unsafe_functions_with_executed_insts\":{},",
                "\"total_function_calls\":{},",
                "\"unsafe_function_calls\":{}",
                "}}"
            ),
            total_insts,
            unsafe_insts,
            unsafe_loads,
            unsafe_stores,
            unsafe_calls_direct,
            unsafe_calls_indirect,
            unsafe_calls_intrinsic,
            unsafe_casts,
            unsafe_geps,
            unsafe_atomics,
            unsafe_others,
            metadata_count,
            unsafe_functions_defined,
            functions_executed,
            unsafe_functions_executed,
            unsafe_functions_with_executed_insts,
            total_function_calls,
            unsafe_function_calls
        );

        let _ = write_stat_json("unsafe_counter", &stats_json);

        if cfg!(debug_assertions) {
            eprintln!("{}", stats_json);
        }
    }
}

// Global tracker instance - const initialized, no allocation
static TRACKER: UnsafeTracker = UnsafeTracker::new();

// ===== C ABI Functions =====

/// Append metadata from one CGU and return the base offset for global IDs.
/// Called by UnsafeFunctionTrackerPass via module constructor (once per CGU).
#[no_mangle]
pub unsafe extern "C" fn __unsafe_init_metadata(metadata_ptr: *const u8, count: u32) -> u32 {
    TRACKER.init_metadata(metadata_ptr, count)
}

/// Record a function call
/// Called by UnsafeFunctionTrackerPass at each function entry
#[no_mangle]
pub unsafe extern "C" fn __unsafe_record_function(func_id: u32) {
    TRACKER.record_function(func_id);
}

/// Record basic block statistics
/// Called by UnsafeInstCounterPass for each basic block
#[no_mangle]
pub unsafe extern "C" fn __unsafe_record_block(
    func_id: u32,
    total: u32,
    unsafe_total: u32,
    unsafe_load: u16,
    unsafe_store: u16,
    unsafe_call_direct: u16,
    unsafe_call_indirect: u16,
    unsafe_call_intrinsic: u16,
    unsafe_cast: u16,
    unsafe_gep: u16,
    unsafe_atomic: u16,
    unsafe_other: u16
) {
    TRACKER.record_block(
        func_id, total, unsafe_total,
        unsafe_load, unsafe_store,
        unsafe_call_direct, unsafe_call_indirect, unsafe_call_intrinsic,
        unsafe_cast, unsafe_gep, unsafe_atomic, unsafe_other
    );
}

/// Dump statistics at program termination
/// Called by UnsafeFunctionTrackerPass via module destructor
#[no_mangle]
pub unsafe extern "C" fn __unsafe_dump_stats() {
    TRACKER.dump_stats();
}

/// Automatic cleanup at program exit (backup)
#[ctor::dtor]
fn cleanup() {
    TRACKER.dump_stats();
}
