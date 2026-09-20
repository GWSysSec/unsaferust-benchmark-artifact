//! Unsafe Rust Performance Monitoring Runtime Library
//!
//! This library provides runtime instrumentation for tracking performance metrics
//! in unsafe Rust code. It integrates with LLVM passes to instrument code and
//! collect statistics about:
//!
//! - Heap memory usage and unsafe memory access patterns
//! - CPU cycle counts for unsafe code blocks  
//! - Source line coverage for unsafe code execution
//! - Unsafe instruction counting and statistics
//!
//! The library is designed to be conditionally compiled based on features to
//! minimize overhead when specific tracking is not needed.
//!
//! ## Features
//! 
//! - `heap_tracker`: Enables heap memory access tracking
//! - `cpu_cycle_counter`: Enables CPU cycle counting (x86_64 only)
//! - `unsafe_coverage`: Enables unsafe code line coverage tracking
//! - `unsafe_counter`: Enables unsafe instruction counting and function statistics
//! 
//! ## Usage
//! 
//! This library is designed to be linked with code instrumented by the corresponding
//! LLVM passes. The runtime functions are automatically called by the instrumented code.
//!
//! For `unsafe_counter`, two LLVM passes work together:
//! - `UnsafeFunctionTrackerPass` (module pass): Tracks function calls and metadata
//! - `UnsafeInstCounterPass` (function pass): Counts unsafe instructions

use std::fs::OpenOptions;
use std::io::{Result as IoResult, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::cell::Cell;

/// Global flag to ensure initialization only happens once across all modules
static RUNTIME_INITIALIZED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Thread-local flag for preventing recursive tracking during internal operations.
    /// This is shared across all modules to ensure consistent behavior.
    pub(crate) static GLOBAL_SKIP_TRACKING: Cell<bool> = Cell::new(false);
}

// ================================================================================================
// SHARED UTILITIES
// ================================================================================================

/// Centralized output writer with error handling and atomic safety.
/// 
/// This function provides a thread-safe way to append content to output files.
/// It's used by all monitoring modules to ensure consistent output behavior.
///
/// # Arguments
/// * `content` - The content to write to the file
/// * `file_path` - The path to the output file
/// 
/// # Returns
/// * `Ok(())` if the write was successful
/// * `Err(io::Error)` if there was an I/O error
pub fn write_output(content: &str, file_path: &str) -> IoResult<()> {
    // Use GLOBAL_SKIP_TRACKING to prevent any allocations during file I/O
    // from being tracked by our monitoring systems
    GLOBAL_SKIP_TRACKING.with(|flag| {
        let was_tracking = flag.get();
        flag.set(true);
        
        let result = (|| {
            let mut output_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)?;
            
            writeln!(output_file, "{}", content)
        })();
        
        flag.set(was_tracking);
        result
    })
}

// ================================================================================================
// STAT PATH + JSON OUTPUT (benchmarking pipeline)
// ================================================================================================

/// Return the basename of the current executable, or "unknown".
fn current_exe_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unknown".into())
}

/// Build the output path for a given metric.
///
/// Format: `$UNSAFE_STAT_DIR/{pid}.{binary}.{metric}.json`
/// Falls back to `/tmp` when `UNSAFE_STAT_DIR` is unset.
pub fn stat_path(metric: &str) -> PathBuf {
    let dir = std::env::var("UNSAFE_STAT_DIR").unwrap_or_else(|_| "/tmp".into());
    let pid = std::process::id();
    let bin = current_exe_name();
    PathBuf::from(dir).join(format!("{}.{}.{}.json", pid, bin, metric))
}

/// Write a JSON stat file with the standard envelope.
///
/// The resulting file contains exactly one JSON object:
/// ```json
/// {"schema":1,"pid":N,"binary":"…","metric":"…","stats":{…}}
/// ```
///
/// Uses `truncate(true)` so each `(pid, binary, metric)` tuple gets a clean
/// file rather than appending to a shared one.
pub fn write_stat_json(metric: &str, stats_json: &str) -> IoResult<()> {
    let path = stat_path(metric);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let pid = std::process::id();
    let bin = current_exe_name();
    let bin = bin.replace('\\', "\\\\").replace('"', "\\\"");

    let json = format!(
        "{{\"schema\":1,\"pid\":{},\"binary\":\"{}\",\"metric\":\"{}\",\"stats\":{}}}",
        pid, bin, metric, stats_json
    );

    GLOBAL_SKIP_TRACKING.with(|flag| {
        let was_tracking = flag.get();
        flag.set(true);

        let result = (|| {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;
            file.write_all(json.as_bytes())
        })();

        flag.set(was_tracking);
        result
    })
}

// ================================================================================================
// RUNTIME INITIALIZATION
// ================================================================================================

/// Initialize the runtime monitoring system.
/// 
/// This function ensures that global initialization only happens once,
/// even if called from multiple modules or threads. It's automatically
/// called during library initialization, but can be called manually if needed.
pub fn initialize_runtime() {
    if RUNTIME_INITIALIZED.compare_exchange(
        false,
        true,
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_err() {
        return; // Already initialized
    }
    
    // Perform any global initialization needed across all modules
    #[cfg(any(feature = "heap_tracker", feature = "cpu_cycle_counter", feature = "unsafe_coverage", feature = "unsafe_counter", feature = "stdlib_api_tracker"))]
    {
        // Initialize thread-local tracking state
        GLOBAL_SKIP_TRACKING.with(|flag| flag.set(false));

        // Install panic-hook protection for the cpu_cycle_counter bracket
        // counters so a panic inside an instrumented unsafe block does not
        // leave IN_UNSAFE permanently > 0 and silently corrupt subsequent
        // measurements on the same thread.
        #[cfg(all(target_arch = "x86_64", feature = "cpu_cycle_counter"))]
        cpu_cycle_counter::install_panic_hook();
    }
}

/// Check if the runtime monitoring system has been initialized.
/// 
/// This function can be used to verify that the runtime is properly set up.
pub fn is_runtime_initialized() -> bool {
    RUNTIME_INITIALIZED.load(Ordering::Acquire)
}

// ================================================================================================
// FEATURE-CONDITIONAL MODULE IMPORTS
// ================================================================================================

#[cfg(feature = "heap_tracker")]
pub mod heap_tracker;

#[cfg(all(target_arch = "x86_64", feature = "cpu_cycle_counter"))]
pub mod cpu_cycle_counter;

#[cfg(feature = "unsafe_coverage")] 
pub mod unsafe_coverage;

#[cfg(feature = "unsafe_counter")]
pub mod unsafe_counter;

#[cfg(feature = "stdlib_api_tracker")]
pub mod stdlib_api_tracker;

// ================================================================================================
// PUBLIC API RE-EXPORTS
// ================================================================================================

// Re-export the main runtime functions that LLVM passes expect to find
#[cfg(feature = "heap_tracker")]
pub use heap_tracker::{
    dyn_mem_access, 
    dyn_unsafe_mem_access,
    // Note: heap_tracker dump_stats is called automatically via dtor
};

#[cfg(all(target_arch = "x86_64", feature = "cpu_cycle_counter"))]
pub use cpu_cycle_counter::{
    // Core functions called by LLVM instrumentation
    record_program_start,           // Called from module constructor
    cpu_cycle_start_measurement,    // Called at unsafe block start
    cpu_cycle_end_measurement,      // Called at unsafe block end

    // External call tracking functions
    external_call_start,            // Called before external function calls
    external_call_end,              // Called after external function calls

    // Stats management functions
    print_cpu_cycle_stats,          // Called from module destructor
};

#[cfg(feature = "unsafe_coverage")]
pub use unsafe_coverage::{
    // Core functions called by LLVM instrumentation (simplified - no line_id!)
    register_unsafe_line,           // Called from module constructor: (line, file)
    track_unsafe_line_execution,    // Called at runtime: (line, file)
    print_unsafe_coverage_stats,    // Called from module destructor
    
    // Additional utility functions for programmatic access
    get_unsafe_coverage_percentage,
    get_registered_unsafe_lines_count,
    get_executed_unsafe_lines_count,
    reset_unsafe_coverage_stats,
};

#[cfg(feature = "stdlib_api_tracker")]
pub use stdlib_api_tracker::{
    __unsafe_record_stdlib_call,
    __unsafe_dump_stdlib_stats,
};

#[cfg(feature = "unsafe_counter")]
pub use unsafe_counter::{
    // Core functions for the two-pass system
    
    // Called by UnsafeFunctionTrackerPass (module pass)
    __unsafe_init_metadata,         // Initialize metadata table from compile-time data
    __unsafe_record_function,       // Record function call at entry
    
    // Called by UnsafeInstCounterPass (function pass)  
    __unsafe_record_block,          // Record basic block statistics
    
    // Called at program termination
    __unsafe_dump_stats,            // Dump final statistics
};

// ================================================================================================
// RUNTIME INITIALIZATION
// ================================================================================================

/// Automatic runtime initialization using ctor.
/// 
/// This ensures the runtime is initialized before any instrumented code runs,
/// regardless of which features are enabled.
#[ctor::ctor]
fn init_unsafe_perf_runtime() {
    initialize_runtime();
}

// ================================================================================================
// TESTING UTILITIES
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_runtime_initialization() {
        // The runtime should be automatically initialized
        assert!(is_runtime_initialized());
    }
    
    #[test]
    fn test_global_skip_tracking() {
        // Test the global skip tracking flag
        GLOBAL_SKIP_TRACKING.with(|flag| {
            assert!(!flag.get()); // Should start as false
            flag.set(true);
            assert!(flag.get());
            flag.set(false);
            assert!(!flag.get());
        });
    }
    
    #[cfg(feature = "heap_tracker")]
    #[test] 
    fn test_heap_tracker_functions_exist() {
        // Test that the heap tracker functions are properly exported
        use std::ptr;
        unsafe {
            dyn_mem_access(ptr::null());
            dyn_unsafe_mem_access(ptr::null(), false);
        }
    }
    
#[cfg(all(target_arch = "x86_64", feature = "cpu_cycle_counter"))]
#[test]
fn test_cpu_cycle_counter_functions() {
    // Record program start
    record_program_start();

    // Test cycle measurement
    let start = cpu_cycle_start_measurement();
    // Do some work
    let mut sum = 0u64;
    for i in 0..100 {
        sum = sum.wrapping_add(i);
    }
    cpu_cycle_end_measurement(start);

    // Test external call tracking
    let ext_start = external_call_start();
    // Simulate some external call work
    for i in 0..50 {
        sum = sum.wrapping_add(i);
    }
    external_call_end(ext_start);

    // Prevent optimization
    assert!(sum > 0);

    // Test stats printing (should not panic)
    print_cpu_cycle_stats();
}
    
#[cfg(feature = "unsafe_coverage")]
#[test]
fn test_unsafe_coverage_functions() {
    use std::ffi::CString;
    
    // Reset stats before testing
    reset_unsafe_coverage_stats();
    
    if let Ok(filename) = CString::new("test.rs") {
        // Test registration (normally called from module constructor)
        // Note: no line_id parameter anymore!
        register_unsafe_line(42, filename.as_ptr());
        
        // Test execution tracking (normally called at runtime)
        track_unsafe_line_execution(42, filename.as_ptr());
        
        // Test coverage query functions
        let percentage = get_unsafe_coverage_percentage();
        assert!(percentage >= 0.0 && percentage <= 100.0);
        
        let registered_count = get_registered_unsafe_lines_count();
        assert!(registered_count >= 1); // We registered at least one line
        
        let executed_count = get_executed_unsafe_lines_count();
        assert!(executed_count >= 1); // We executed at least one line
        
        // Since we registered and executed the same line, coverage should be 100%
        assert_eq!(percentage, 100.0);
        
        // Test stats printing (normally called from module destructor)
        print_unsafe_coverage_stats();
    }
}
    
    #[cfg(feature = "unsafe_counter")]
    #[test]
    fn test_unsafe_counter_functions() {
        use std::ptr;
        
        // Test the two-pass system functions
        unsafe {
            // Test metadata initialization (normally called by module constructor)
            // We'll pass a null pointer with 0 count to test the function exists
            __unsafe_init_metadata(ptr::null(), 0);
            
            // Test function recording (normally called at function entry)
            __unsafe_record_function(0);
            
            // Test block recording (normally called per basic block)
            __unsafe_record_block(
                0,    // func_id
                100,  // total instructions
                10,   // unsafe instructions
                2,    // unsafe loads
                1,    // unsafe stores
                3,    // unsafe calls
                1,    // unsafe casts
                2,    // unsafe GEPs
                1     // unsafe others
            );
            
            // Test stats dumping (normally called at program exit)
            __unsafe_dump_stats();
        }
    }
    
    #[cfg(feature = "unsafe_counter")]
    #[test]
    fn test_unsafe_counter_two_pass_workflow() {
        use std::ptr;
        
        unsafe {
            // Simulate the workflow of the two-pass system
            
            // 1. Module pass initializes metadata (empty for this test)
            __unsafe_init_metadata(ptr::null(), 0);
            
            // 2. Module pass records function entries
            __unsafe_record_function(1);
            __unsafe_record_function(2);
            __unsafe_record_function(1); // Same function called twice
            
            // 3. Function pass records basic block stats
            __unsafe_record_block(1, 50, 5, 2, 1, 1, 0, 1, 0);
            __unsafe_record_block(1, 30, 0, 0, 0, 0, 0, 0, 0); // Safe block
            __unsafe_record_block(2, 40, 8, 3, 2, 0, 1, 1, 1);
            
            // 4. Stats are dumped at exit (this should not panic)
            __unsafe_dump_stats();
        }
    }
    
    #[test]
    fn test_write_output() {
        // Test that write_output works without panicking
        let result = write_output("test content", "/tmp/test_output.tmp");
        // We don't assert success since /tmp might not exist on all systems,
        // but it should not panic
        let _ = result;
    }
}