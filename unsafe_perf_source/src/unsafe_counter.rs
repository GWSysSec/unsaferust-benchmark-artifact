//! Runtime library for UnsafeCount LLVM passes
//!
//! Lock-free, zero-allocation runtime supporting the two-pass system:
//! - UnsafeFunctionTrackerPass (module pass): tracks function calls
//! - UnsafeInstCounterPass (function pass): counts unsafe instructions

use std::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};
use std::mem::MaybeUninit;
use crate::write_output;

/// Maximum number of functions we can track
const MAX_FUNCTIONS: usize = 65536;

/// Function metadata from compile-time analysis
#[repr(C)]
#[derive(Copy, Clone)]
struct FunctionMetadata {
    id: u32,
    has_unsafe_inst: u8,    // Has !unsafe_inst metadata
    has_unsafe_regions: u8,  // Has unsafe marker pairs
    _padding: u16,
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
    
    // ===== Instruction Counting (from UnsafeInstCounterPass) =====
    
    // Global instruction counters
    total_instructions: AtomicU64,
    total_unsafe_instructions: AtomicU64,
    
    // Unsafe instruction type counters (6 categories)
    unsafe_loads: AtomicU64,
    unsafe_stores: AtomicU64,
    unsafe_calls: AtomicU64,
    unsafe_casts: AtomicU64,
    unsafe_geps: AtomicU64,
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
            
            // Instruction counting
            total_instructions: AtomicU64::new(0),
            total_unsafe_instructions: AtomicU64::new(0),
            unsafe_loads: AtomicU64::new(0),
            unsafe_stores: AtomicU64::new(0),
            unsafe_calls: AtomicU64::new(0),
            unsafe_casts: AtomicU64::new(0),
            unsafe_geps: AtomicU64::new(0),
            unsafe_others: AtomicU64::new(0),
            
            // Control
            stats_written: AtomicBool::new(false),
            metadata_initialized: AtomicBool::new(false),
        }
    }
    
    // ===== Functions called by UnsafeFunctionTrackerPass =====
    
    /// Initialize metadata table from compile-time data
    /// Called once at program startup by module constructor
    unsafe fn init_metadata(&self, metadata_ptr: *const u8, count: u32) {
        // Ensure single initialization
        if self.metadata_initialized.swap(true, Ordering::AcqRel) {
            return;
        }
        
        if count > MAX_FUNCTIONS as u32 {
            eprintln!("Warning: Function count {} exceeds maximum {}", count, MAX_FUNCTIONS);
            return;
        }
        
        let metadata_slice = std::slice::from_raw_parts(
            metadata_ptr as *const FunctionMetadata,
            count as usize
        );

        let meta_ptr = self.metadata.as_ptr() as *mut MaybeUninit<FunctionMetadata>;
        
        for (i, meta) in metadata_slice.iter().enumerate() {
            unsafe {
                (*meta_ptr.add(i)).write(*meta);
            }
        }

        self.metadata_count.store(count, Ordering::Release);
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
        _func_id: u32,  // Available but not needed
        total: u32,
        unsafe_total: u32,
        unsafe_load: u16,
        unsafe_store: u16,
        unsafe_call: u16,
        unsafe_cast: u16,
        unsafe_gep: u16,
        unsafe_other: u16
    ) {
        // Update global counters
        self.total_instructions.fetch_add(total as u64, Ordering::Relaxed);
        
        // Early exit if no unsafe instructions
        if unsafe_total == 0 {
            return;
        }
        
        self.total_unsafe_instructions.fetch_add(unsafe_total as u64, Ordering::Relaxed);
        
        // Update category counters only if non-zero
        if unsafe_load > 0 {
            self.unsafe_loads.fetch_add(unsafe_load as u64, Ordering::Relaxed);
        }
        if unsafe_store > 0 {
            self.unsafe_stores.fetch_add(unsafe_store as u64, Ordering::Relaxed);
        }
        if unsafe_call > 0 {
            self.unsafe_calls.fetch_add(unsafe_call as u64, Ordering::Relaxed);
        }
        if unsafe_cast > 0 {
            self.unsafe_casts.fetch_add(unsafe_cast as u64, Ordering::Relaxed);
        }
        if unsafe_gep > 0 {
            self.unsafe_geps.fetch_add(unsafe_gep as u64, Ordering::Relaxed);
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
        let unsafe_calls_inst = self.unsafe_calls.load(Ordering::Relaxed);
        let unsafe_casts = self.unsafe_casts.load(Ordering::Relaxed);
        let unsafe_geps = self.unsafe_geps.load(Ordering::Relaxed);
        let unsafe_others = self.unsafe_others.load(Ordering::Relaxed);
        
        // Calculate function statistics
        let mut unique_functions = 0u32;
        let mut unique_unsafe_functions = 0u32;
        let mut total_function_calls = 0u64;
        let mut unsafe_function_calls = 0u64;
        
        for i in 0..metadata_count {
            if self.functions_seen.is_set(i) {
                unique_functions += 1;
                
                // Get metadata for this function
                let meta = unsafe { self.metadata[i].assume_init() };
                let is_unsafe = meta.has_unsafe_inst != 0 || meta.has_unsafe_regions != 0;
                
                if is_unsafe {
                    unique_unsafe_functions += 1;
                }
                
                // Get call count
                let calls = self.function_calls[i].value.load(Ordering::Relaxed);
                total_function_calls += calls;
                
                if is_unsafe {
                    unsafe_function_calls += calls;
                }
            }
        }
        
        // Format output in simple format
        let output = format!(
            concat!(
                "Total instructions: {}\n",
                "Unsafe instructions: {}\n",
                "Unsafe loads: {}\n",
                "Unsafe stores: {}\n",
                "Unsafe calls: {}\n",
                "Unsafe casts: {}\n",
                "Unsafe GEPs: {}\n",
                "Unsafe others: {}\n",
                "Unique functions: {}\n",
                "Unique unsafe functions: {}\n",
                "Total function calls: {}\n",
                "Unsafe function calls: {}\n"
            ),
            total_insts,
            unsafe_insts,
            unsafe_loads,
            unsafe_stores,
            unsafe_calls_inst,
            unsafe_casts,
            unsafe_geps,
            unsafe_others,
            unique_functions,
            unique_unsafe_functions,
            total_function_calls,
            unsafe_function_calls
        );
        
        // Write to file
        let _ = write_output(&output, "unsafe_counter.stat");
        
        if cfg!(debug_assertions) {
            eprintln!("{}", output);
        }
    }
}

// Global tracker instance - const initialized, no allocation
static TRACKER: UnsafeTracker = UnsafeTracker::new();

// ===== C ABI Functions =====

/// Initialize metadata table from compile-time data
/// Called by UnsafeFunctionTrackerPass via module constructor
#[no_mangle]
pub unsafe extern "C" fn __unsafe_init_metadata(metadata_ptr: *const u8, count: u32) {
    TRACKER.init_metadata(metadata_ptr, count);
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
    unsafe_call: u16,
    unsafe_cast: u16,
    unsafe_gep: u16,
    unsafe_other: u16
) {
    TRACKER.record_block(
        func_id, total, unsafe_total,
        unsafe_load, unsafe_store, unsafe_call,
        unsafe_cast, unsafe_gep, unsafe_other
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