//! Unsafe Line Coverage Runtime Library
//!
//! Reports per run:
//!   unsafe_source_lines_pre_opt   — lines InstMarker found in unsafe blocks
//!                                   BEFORE LLVM optimization ran (accumulated
//!                                   across all CGUs via fetch_add in the
//!                                   module ctor emitted by DynamicLineCount)
//!   unsafe_source_lines_total     — subset of pre_opt whose IR SURVIVED
//!                                   optimization (the coverage denominator)
//!   unsafe_source_lines_eliminated — pre_opt - total
//!   unsafe_source_lines_executed  — subset of total actually hit at runtime
//!   unsafe_line_coverage_pct      — executed / total
//!
//! Storage is DashSet<String> (lock-free sharded set) rather than
//! Mutex<HashSet>; the previous mutex became a contention hotspot in
//! multi-threaded benchmarks that spent any time inside unsafe blocks.

use dashmap::DashSet;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use crate::write_stat_json;

struct UnsafeCoverageTracker {
    registered_lines: DashSet<String>,
    executed_lines: DashSet<String>,
    // Pre-optimization unsafe source line count, summed across all CGUs.
    // NOTE: this is a *sum* of per-module line counts, not a union, so lines
    // that appear in multiple CGUs are counted once per CGU. It gives an
    // upper-bound on elimination, which is what we want for the "lines
    // eliminated by LLVM" diagnostic.
    pre_opt_total: AtomicU64,
    stats_written: AtomicBool,
    run_counter: AtomicUsize,
}

impl UnsafeCoverageTracker {
    fn new() -> Self {
        Self {
            registered_lines: DashSet::new(),
            executed_lines: DashSet::new(),
            pre_opt_total: AtomicU64::new(0),
            stats_written: AtomicBool::new(false),
            run_counter: AtomicUsize::new(0),
        }
    }

    fn add_pre_opt_count(&self, count: u64) {
        self.pre_opt_total.fetch_add(count, Ordering::Relaxed);
    }

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

    fn register_line(&self, line: i64, file: *const c_char) {
        self.registered_lines.insert(Self::make_location(line, file));
    }

    fn track_execution(&self, line: i64, file: *const c_char) {
        self.executed_lines.insert(Self::make_location(line, file));
    }

    fn get_coverage_percentage(&self) -> f64 {
        let total = self.registered_lines.len();
        let executed = self.executed_lines.len();
        if total > 0 {
            (executed as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    fn get_registered_count(&self) -> usize {
        self.registered_lines.len()
    }

    fn get_executed_count(&self) -> usize {
        self.executed_lines.len()
    }

    fn reset(&self) {
        self.registered_lines.clear();
        self.executed_lines.clear();
        self.pre_opt_total.store(0, Ordering::Release);
        self.stats_written.store(false, Ordering::Release);
        self.run_counter.store(0, Ordering::Release);
    }

    fn write_stats(&self) {
        if self.stats_written.swap(true, Ordering::AcqRel) {
            return;
        }

        let pre_opt = self.pre_opt_total.load(Ordering::Acquire);
        let total = self.registered_lines.len() as u64;
        let executed = self.executed_lines.len() as u64;
        // `pre_opt` is a cross-CGU sum and `total` is a cross-CGU union, so
        // `pre_opt >= total` holds in the common single-CGU case but may not
        // with multiple CGUs (same line counted N times pre-opt, deduped to 1
        // in the survived set). `saturating_sub` gives 0 instead of wrapping.
        let eliminated = pre_opt.saturating_sub(total);
        let coverage = if total > 0 {
            (executed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        eprintln!(
            "unsafe_source_lines_pre_opt: {}\n\
             unsafe_source_lines_total: {}\n\
             unsafe_source_lines_eliminated: {}\n\
             unsafe_source_lines_executed: {}\n\
             unsafe_line_coverage_pct: {:.2}",
            pre_opt, total, eliminated, executed, coverage
        );

        let mut registered_vec: Vec<String> =
            self.registered_lines.iter().map(|e| e.clone()).collect();
        registered_vec.sort();
        let mut executed_vec: Vec<String> =
            self.executed_lines.iter().map(|e| e.clone()).collect();
        executed_vec.sort();

        let registered_json = Self::lines_to_json_array(&registered_vec);
        let executed_json = Self::lines_to_json_array(&executed_vec);

        let stats_json = format!(
            concat!(
                "{{",
                "\"unsafe_source_lines_pre_opt\":{},",
                "\"unsafe_source_lines_total\":{},",
                "\"unsafe_source_lines_eliminated\":{},",
                "\"unsafe_source_lines_executed\":{},",
                "\"unsafe_line_coverage_pct\":{:.2},",
                "\"registered_lines\":{},",
                "\"executed_lines\":{}",
                "}}"
            ),
            pre_opt, total, eliminated, executed, coverage,
            registered_json, executed_json
        );

        let _ = write_stat_json("coverage", &stats_json);
    }

    fn lines_to_json_array(lines: &[String]) -> String {
        let escaped: Vec<String> = lines.iter()
            .map(|l| format!("\"{}\"", l.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        format!("[{}]", escaped.join(","))
    }
}

static COVERAGE_TRACKER: once_cell::sync::Lazy<UnsafeCoverageTracker> =
    once_cell::sync::Lazy::new(UnsafeCoverageTracker::new);

// ===== C-ABI Public Interface =====

#[no_mangle]
pub extern "C" fn register_unsafe_line(line: i64, file: *const c_char) {
    COVERAGE_TRACKER.register_line(line, file);
}

/// Record this module's pre-optimization unsafe source line count.
/// Called once per CGU from the module constructor emitted by DynamicLineCount.
#[no_mangle]
pub extern "C" fn register_unsafe_lines_pre_opt_count(count: u64) {
    COVERAGE_TRACKER.add_pre_opt_count(count);
}

#[no_mangle]
pub extern "C" fn track_unsafe_line_execution(line: i64, file: *const c_char) {
    COVERAGE_TRACKER.track_execution(line, file);
}

#[no_mangle]
pub extern "C" fn print_unsafe_coverage_stats() {
    COVERAGE_TRACKER.write_stats();
}

#[no_mangle]
pub extern "C" fn get_unsafe_coverage_percentage() -> f64 {
    COVERAGE_TRACKER.get_coverage_percentage()
}

#[no_mangle]
pub extern "C" fn get_registered_unsafe_lines_count() -> usize {
    COVERAGE_TRACKER.get_registered_count()
}

#[no_mangle]
pub extern "C" fn get_executed_unsafe_lines_count() -> usize {
    COVERAGE_TRACKER.get_executed_count()
}

#[no_mangle]
pub extern "C" fn reset_unsafe_coverage_stats() {
    COVERAGE_TRACKER.reset();
}

#[ctor::dtor]
fn dump_coverage_at_exit() {
    COVERAGE_TRACKER.write_stats();
}
