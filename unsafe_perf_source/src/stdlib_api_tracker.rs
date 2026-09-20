//! Runtime library for StdlibApiTracker LLVM pass
//!
//! Tracks per-API invocation counts for stdlib calls inside unsafe code.
//! Uses DashMap for lock-free concurrent tracking across threads.

use std::sync::atomic::{AtomicBool, Ordering};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use crate::write_stat_json;

/// Per-API invocation counter using DashMap for lock-free concurrent access.
/// Key: API path string (e.g. "std::vec::Vec::<T, A>::set_len")
/// Value: invocation count
static API_COUNTS: Lazy<DashMap<String, u64>> = Lazy::new(DashMap::new);

static STATS_WRITTEN: AtomicBool = AtomicBool::new(false);

/// Record a stdlib API call from instrumented code.
///
/// Called by StdlibApiTrackerPass at each stdlib call site inside an unsafe SESE region.
/// The API path is passed as a pointer+length pair (not null-terminated).
#[no_mangle]
pub unsafe extern "C" fn __unsafe_record_stdlib_call(api_ptr: *const u8, api_len: u32) {
    if api_ptr.is_null() || api_len == 0 {
        return;
    }

    let api_bytes = std::slice::from_raw_parts(api_ptr, api_len as usize);
    let api_str = match std::str::from_utf8(api_bytes) {
        Ok(s) => s,
        Err(_) => return,
    };

    API_COUNTS
        .entry(api_str.to_owned())
        .and_modify(|count| *count += 1)
        .or_insert(1);
}

/// Dump per-API statistics to the stat file.
///
/// Called at program exit via #[ctor::dtor]. Only writes once.
fn dump_stdlib_stats() {
    if STATS_WRITTEN.swap(true, Ordering::AcqRel) {
        return;
    }

    if API_COUNTS.is_empty() {
        return;
    }

    let mut entries: Vec<(String, u64)> = API_COUNTS
        .iter()
        .map(|entry| (entry.key().clone(), *entry.value()))
        .collect();

    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let total_calls: u64 = entries.iter().map(|(_, c)| c).sum();
    let unique_apis = entries.len();

    let api_entries: Vec<String> = entries.iter()
        .map(|(api, count)| {
            let escaped = api.replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{}\":{}", escaped, count)
        })
        .collect();

    let stats_json = format!(
        "{{\"total_calls\":{},\"unique_apis\":{},\"api_calls\":{{{}}}}}",
        total_calls, unique_apis, api_entries.join(",")
    );

    let _ = write_stat_json("stdlib_api", &stats_json);

    if cfg!(debug_assertions) {
        eprint!("{}", stats_json);
    }
}

#[no_mangle]
pub unsafe extern "C" fn __unsafe_dump_stdlib_stats() {
    dump_stdlib_stats();
}

#[ctor::dtor]
fn cleanup() {
    dump_stdlib_stats();
}
