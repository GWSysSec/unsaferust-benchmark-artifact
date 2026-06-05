//! Unsafe Line Coverage Runtime Library
//! Track: total unsafe lines (compilation) vs executed unsafe lines (runtime)
//! Simplified implementation using direct file:line tracking

use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// Simple coverage tracker - just track file:line strings
struct UnsafeCoverageTracker {
    // Simple HashSets for registered and executed lines
    registered_lines: Mutex<HashSet<String>>,
    executed_lines: Mutex<HashSet<String>>,

    // Flag to ensure stats are written only once
    stats_written: AtomicBool,

    // Run counter for multiple main() executions
    run_counter: AtomicUsize,
}

impl UnsafeCoverageTracker {
    fn new() -> Self {
        Self {
            registered_lines: Mutex::new(HashSet::new()),
            executed_lines: Mutex::new(HashSet::new()),
            stats_written: AtomicBool::new(false),
            run_counter: AtomicUsize::new(0),
        }
    }
    
    /// Convert C string + line to a location string
    fn make_location(line: i64, file: *const c_char) -> String {
        unsafe {
            if file.is_null() {
                format!("<unknown>:{}", line)
            } else {
                match CStr::from_ptr(file).to_str() {
                    Ok(s) => format!("{}:{}", s, line),
                    Err(_) => format!("<invalid>:{}", line),
                }
            }
        }
    }
    
    /// Register an unsafe line found at compile time
    fn register_line(&self, line: i64, file: *const c_char) {
        let location = Self::make_location(line, file);
        self.registered_lines.lock().unwrap().insert(location);
    }
    
    /// Track execution of an unsafe line at runtime
    fn track_execution(&self, line: i64, file: *const c_char) {
        let location = Self::make_location(line, file);
        self.executed_lines.lock().unwrap().insert(location);
    }
    
    /// Get coverage percentage
    fn get_coverage_percentage(&self) -> f64 {
        let registered = self.registered_lines.lock().unwrap();
        let executed = self.executed_lines.lock().unwrap();
        
        let registered_count = registered.len();
        let executed_count = executed.len();
        
        if registered_count > 0 {
            (executed_count as f64 / registered_count as f64) * 100.0
        } else {
            0.0
        }
    }
    
    /// Get number of registered lines
    fn get_registered_count(&self) -> usize {
        self.registered_lines.lock().unwrap().len()
    }
    
    /// Get number of executed lines
    fn get_executed_count(&self) -> usize {
        self.executed_lines.lock().unwrap().len()
    }
    
    /// Reset all statistics
    fn reset(&self) {
        self.registered_lines.lock().unwrap().clear();
        self.executed_lines.lock().unwrap().clear();
        self.stats_written.store(false, Ordering::Release);
        self.run_counter.store(0, Ordering::Release);
    }
    
    /// Write statistics to file and stderr
    fn write_stats(&self) {
        // Ensure single execution
        if self.stats_written.swap(true, Ordering::AcqRel) {
            return;
        }

        let registered = self.registered_lines.lock().unwrap();
        let executed = self.executed_lines.lock().unwrap();

        let registered_count = registered.len();
        let executed_count = executed.len();
        let coverage = if registered_count > 0 {
            (executed_count as f64 / registered_count as f64) * 100.0
        } else {
            0.0
        };

        // Increment run counter
        let run_num = self.run_counter.fetch_add(1, Ordering::AcqRel) + 1;

        // Print to stderr (simple coverage percentage only)
        eprintln!("Coverage: {:.2}%", coverage);

        // Append to file with new format
        self.write_detailed_stats(run_num, &registered, &executed, registered_count, executed_count, coverage);
    }

    /// Write detailed statistics to file in new format
    fn write_detailed_stats(&self, run_num: usize, registered: &HashSet<String>, executed: &HashSet<String>,
                           registered_count: usize, executed_count: usize, coverage: f64) {
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut output = format!("=== RUN_{} ===\n", run_num);

        // Registered lines section
        output.push_str("=== REGISTERED_LINES ===\n");
        let mut registered_vec: Vec<_> = registered.iter().collect();
        registered_vec.sort();
        for line in registered_vec {
            output.push_str(&format!("{}\n", line));
        }
        output.push_str("\n");

        // Executed lines section
        output.push_str("=== EXECUTED_LINES ===\n");
        let mut executed_vec: Vec<_> = executed.iter().collect();
        executed_vec.sort();
        for line in executed_vec {
            output.push_str(&format!("{}\n", line));
        }
        output.push_str("\n");

        // Summary section
        output.push_str("=== SUMMARY ===\n");
        output.push_str(&format!("registered_count={}\n", registered_count));
        output.push_str(&format!("executed_count={}\n", executed_count));
        output.push_str(&format!("coverage_percentage={:.2}\n", coverage));
        output.push_str(&format!("run_timestamp={}\n", timestamp));
        output.push_str("\n");

        use crate::write_output;
        let _ = write_output(&output, "unsafe_coverage.stat");
    }
}

// Global tracker instance
static COVERAGE_TRACKER: once_cell::sync::Lazy<UnsafeCoverageTracker> = 
    once_cell::sync::Lazy::new(|| UnsafeCoverageTracker::new());

// ===== C-ABI Public Interface =====

/// Register an unsafe line found at compile time
#[no_mangle]
pub extern "C" fn register_unsafe_line(line: i64, file: *const c_char) {
    COVERAGE_TRACKER.register_line(line, file);
}

/// Track execution of an unsafe line at runtime
#[no_mangle]
pub extern "C" fn track_unsafe_line_execution(line: i64, file: *const c_char) {
    COVERAGE_TRACKER.track_execution(line, file);
}

/// Print coverage statistics
#[no_mangle]
pub extern "C" fn print_unsafe_coverage_stats() {
    COVERAGE_TRACKER.write_stats();
}

/// Get coverage percentage
#[no_mangle]
pub extern "C" fn get_unsafe_coverage_percentage() -> f64 {
    COVERAGE_TRACKER.get_coverage_percentage()
}

/// Get registered lines count
#[no_mangle]
pub extern "C" fn get_registered_unsafe_lines_count() -> usize {
    COVERAGE_TRACKER.get_registered_count()
}

/// Get executed lines count
#[no_mangle]
pub extern "C" fn get_executed_unsafe_lines_count() -> usize {
    COVERAGE_TRACKER.get_executed_count()
}

/// Reset coverage statistics
#[no_mangle]
pub extern "C" fn reset_unsafe_coverage_stats() {
    COVERAGE_TRACKER.reset();
}

/// Dump stats at program termination
#[ctor::dtor]
fn dump_coverage_at_exit() {
    COVERAGE_TRACKER.write_stats();
}