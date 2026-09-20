//! Runtime library for CPU cycle tracking.
//!
//! Tracks four time categories:
//! - Total cycles:          Entire program execution
//! - Unsafe cycles (total): Time spent in unsafe blocks — INCLUDING time
//!                          spent in external calls made from unsafe code
//! - Unsafe cycles (ext):   The subset of unsafe cycles spent inside
//!                          external/FFI calls invoked from unsafe code
//!                          (markers inserted by ExternalCallTracker into SESE
//!                          regions via unsafe_external_call_start/end)
//! - External cycles:       Time spent in external calls from SAFE code only
//!
//! Derived: unsafe_cycles_internal = unsafe_cycles_total - unsafe_cycles_external
//!          (time unsafe blocks spent executing their *own* instructions
//!           rather than dispatching to an external callee)
//!
//! INVARIANT (Fix A): unsafe_cycles_external <= unsafe_cycles_total, always.
//! Earlier versions timed the two as independent rdtsc brackets that committed
//! at different times: the external sub-interval committed as each inner call
//! returned, while the parent unsafe interval committed only when the outermost
//! unsafe block closed. A stats snapshot taken while a worker thread was parked
//! INSIDE an open unsafe block (the park/futex being the instrumented external
//! call) then saw the child committed but not the parent — so summed across a
//! thread pool, unsafe_external could exceed unsafe_total. This appeared in
//! every thread-pool crate (loom/rayon-core/tokio/ring/jpeg-decoder) and nowhere
//! else. Now unsafe_external_call_end accumulates into a per-thread, per-frame
//! cell (UNSAFE_FRAME_EXT_ACCUM); the outermost unsafe block commits BOTH
//! counters from the SAME bracket at frame close — parent FIRST, then
//! min(child_accum, parent_delta). The per-frame clamp bounds the child by its
//! own parent, and committing parent-before-child means any concurrent snapshot
//! sees unsafe_total >= unsafe_external. No downstream clamp is applied: if the
//! invariant ever breaks again it must surface in the raw data, not be masked.
//!
//! Clock (Fix B): brackets open with `lfence; rdtsc; lfence` (read_tsc_start)
//! and close with `rdtscp; lfence` (read_tsc_end). A bare rdtsc is not ordered
//! w.r.t. surrounding code, and the LLVM-inserted SeqCst fence is only a *memory*
//! fence — it does not pin the rdtsc instruction. The serializing variants stop
//! the counter read from drifting into/out of the region, removing the
//! small-region downward bias caused by silently dropped negative deltas.
//!
//! Callback handling: when an external call invokes a user callback containing
//! unsafe code (e.g., qsort with an unsafe comparator), the callback's unsafe
//! time is subtracted from external_cycles to prevent double-counting.
//!
//! Calculation: unsafe% = unsafe / (total - external)

use ctor::dtor;
use lazy_static::lazy_static;
use std::cell::Cell;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use crate::write_stat_json;

const MAX_THREADS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
enum ThreadState {
    Uninitialized,
    Active,
    Terminated,
}

#[repr(C, align(64))] // Cache-line aligned to prevent false sharing
struct ThreadStats {
    thread_id: AtomicU64,
    state: AtomicUsize, // Stores ThreadState as usize
    start_tsc: AtomicU64,

    // Four cycle counters
    total_cycles: AtomicU64,            // Total program execution
    unsafe_cycles: AtomicU64,           // Time in unsafe blocks (TOTAL)
    unsafe_external_cycles: AtomicU64,  // Subset of unsafe_cycles spent in external calls
    external_cycles: AtomicU64,         // External calls from safe code only

    // Block counts
    unsafe_blocks: AtomicU64,
    external_calls: AtomicU64,
    unsafe_external_calls: AtomicU64,

    _padding: [u64; 1],
}

impl ThreadStats {
    const fn new() -> Self {
        Self {
            thread_id: AtomicU64::new(0),
            state: AtomicUsize::new(ThreadState::Uninitialized as usize),
            start_tsc: AtomicU64::new(0),
            total_cycles: AtomicU64::new(0),
            unsafe_cycles: AtomicU64::new(0),
            unsafe_external_cycles: AtomicU64::new(0),
            external_cycles: AtomicU64::new(0),
            unsafe_blocks: AtomicU64::new(0),
            external_calls: AtomicU64::new(0),
            unsafe_external_calls: AtomicU64::new(0),
            _padding: [0; 1],
        }
    }
}

struct ThreadRegistry {
    threads: [ThreadStats; MAX_THREADS],
    next_slot: AtomicUsize,
    stats_written: AtomicBool,
}

impl ThreadRegistry {
    const fn new() -> Self {
        Self {
            threads: [const { ThreadStats::new() }; MAX_THREADS],
            next_slot: AtomicUsize::new(0),
            stats_written: AtomicBool::new(false),
        }
    }

    fn allocate_slot(&self) -> Option<usize> {
        // First, try to reuse a terminated thread slot
        let current_slots = self.next_slot.load(Ordering::Acquire);
        for slot in 0..current_slots.min(MAX_THREADS) {
            let stats = &self.threads[slot];
            let current_state = stats.state.load(Ordering::Acquire);

            // Try to atomically change from Terminated to Uninitialized for reuse
            if current_state == ThreadState::Terminated as usize {
                let swap_result = stats.state.compare_exchange(
                    ThreadState::Terminated as usize,
                    ThreadState::Uninitialized as usize,
                    Ordering::AcqRel,
                    Ordering::Acquire
                );

                if swap_result.is_ok() {
                    // Successfully claimed a terminated slot for reuse, reset its statistics
                    stats.thread_id.store(0, Ordering::Relaxed);
                    stats.start_tsc.store(0, Ordering::Relaxed);
                    stats.total_cycles.store(0, Ordering::Relaxed);
                    stats.unsafe_cycles.store(0, Ordering::Relaxed);
                    stats.unsafe_external_cycles.store(0, Ordering::Relaxed);
                    stats.external_cycles.store(0, Ordering::Relaxed);
                    stats.unsafe_blocks.store(0, Ordering::Relaxed);
                    stats.external_calls.store(0, Ordering::Relaxed);
                    stats.unsafe_external_calls.store(0, Ordering::Relaxed);
                    return Some(slot);
                }
            }
        }

        // No terminated slots available, try to allocate a new one
        let slot = self.next_slot.fetch_add(1, Ordering::Relaxed);
        if slot < MAX_THREADS {
            Some(slot)
        } else {
            self.next_slot.fetch_sub(1, Ordering::Relaxed);
            None
        }
    }
}

static REGISTRY: ThreadRegistry = ThreadRegistry::new();

// Simple thread-local state tracking
thread_local! {
    static THREAD_SLOT: Cell<Option<usize>> = Cell::new(None);
    static IN_UNSAFE: Cell<u32> = Cell::new(0);
    static IN_EXTERNAL: Cell<u32> = Cell::new(0);
    // Nesting depth for external calls that occurred inside an unsafe region.
    // Incremented/decremented by unsafe_external_call_start/end. Needed so
    // only the outermost such call is timed (nested calls share the same
    // outer TSC interval).
    static IN_UNSAFE_EXT: Cell<u32> = Cell::new(0);
    // Accumulates unsafe cycles that occurred inside an external call (callback scenario).
    // Subtracted from external_cycles at external_call_end to prevent double-counting.
    static UNSAFE_IN_EXT_ACCUM: Cell<u64> = Cell::new(0);
    // Fix A: cycles spent in external calls made inside the CURRENT outermost
    // unsafe frame. unsafe_external_call_end adds here instead of committing to
    // the global counter; cpu_cycle_end_measurement commits it (clamped to the
    // frame's own delta) when the outermost unsafe block closes, so
    // unsafe_external can never exceed unsafe_total.
    static UNSAFE_FRAME_EXT_ACCUM: Cell<u64> = Cell::new(0);
}

/// Open a timing bracket. `lfence; rdtsc; lfence` keeps the counter read from
/// floating above the start of the region (the leading lfence drains prior
/// instructions; the trailing one stops the rdtsc from being reordered after
/// the region's first instructions). See the module-level "Clock (Fix B)" note.
#[inline(always)]
fn read_tsc_start() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use core::arch::x86_64::{_mm_lfence, _rdtsc};
        _mm_lfence();
        let t = _rdtsc();
        _mm_lfence();
        t
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Close a timing bracket. `rdtscp` is partially serializing — it waits for all
/// prior instructions to retire before reading the counter — and the trailing
/// `lfence` stops later instructions from racing the read backwards across the
/// region end. `aux` (the CPU/socket signature) is read but unused; capturing
/// it here leaves room to detect cross-core TSC migration later.
#[inline(always)]
fn read_tsc_end() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use core::arch::x86_64::{_mm_lfence, __rdtscp};
        let mut aux: u32 = 0;
        let t = __rdtscp(&mut aux as *mut u32);
        let _ = aux;
        _mm_lfence();
        t
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Initializes tracking for the current thread, allocating a slot in the registry.
fn initialize_thread() -> Option<usize> {
    THREAD_SLOT.with(|slot_cell| {
        if let Some(slot) = slot_cell.get() {
            return Some(slot);
        }

        if let Some(slot) = REGISTRY.allocate_slot() {
            let tsc = read_tsc_start();
            let stats = &REGISTRY.threads[slot];
            stats.thread_id.store(get_thread_id(), Ordering::Relaxed);
            stats.start_tsc.store(tsc, Ordering::Release);
            stats.state.store(ThreadState::Active as usize, Ordering::Release);

            // Initialize state
            IN_UNSAFE.with(|in_unsafe| in_unsafe.set(0));
            IN_EXTERNAL.with(|in_external| in_external.set(0));
            IN_UNSAFE_EXT.with(|d| d.set(0));
            UNSAFE_IN_EXT_ACCUM.with(|acc| acc.set(0));
            UNSAFE_FRAME_EXT_ACCUM.with(|acc| acc.set(0));

            slot_cell.set(Some(slot));
            Some(slot)
        } else {
            eprintln!("[Runtime] Error: Maximum number of threads ({}) exceeded.", MAX_THREADS);
            None
        }
    })
}

fn get_thread_id() -> u64 {
    #[cfg(target_family = "unix")]
    {
        unsafe { libc::pthread_self() as u64 }
    }
    #[cfg(not(target_family = "unix"))]
    {
        // This is not a stable ID but is a reasonable fallback.
        std::thread::current().id().as_u64().get()
    }
}

/// Marks the current thread as terminated and records its final cycles.
fn thread_cleanup() {
    if let Some(slot) = THREAD_SLOT.with(|s| s.get()) {
        if slot < MAX_THREADS {
            let final_tsc = read_tsc_end();
            let stats = &REGISTRY.threads[slot];

            // Calculate total cycles for this thread
            let start_tsc = stats.start_tsc.load(Ordering::Acquire);
            if final_tsc > start_tsc {
                let total = final_tsc - start_tsc;
                stats.total_cycles.store(total, Ordering::Release);
            }

            stats.state.store(ThreadState::Terminated as usize, Ordering::Release);
        }
    }
}

// ==========================================================================================
// === C ABI Functions for LLVM Pass
// ==========================================================================================

#[no_mangle]
pub extern "C" fn record_program_start() {
    initialize_thread();
}

/// Install a panic hook that resets this thread's nesting-depth counters
/// when a panic unwinds out of an instrumented unsafe / external-call block.
///
/// Without this, a `panic!` inside an unsafe block skips the matching
/// `cpu_cycle_end_measurement`, leaving `IN_UNSAFE` permanently > 0;
/// every subsequent measurement on that thread is then misclassified as
/// nested and its rdtsc bracket is dropped, silently corrupting cycle
/// counts across the rest of the test binary.
///
/// Idempotent: safe to call multiple times. The previous hook is chained
/// so the default panic-printing behaviour (or any harness hook) still runs.
pub fn install_panic_hook() {
    use std::sync::Once;
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let prior = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Reset on a best-effort basis; `.try_with` avoids re-panicking
            // if TLS is being torn down during thread exit.
            let _ = IN_UNSAFE.try_with(|d| d.set(0));
            let _ = IN_EXTERNAL.try_with(|d| d.set(0));
            let _ = IN_UNSAFE_EXT.try_with(|d| d.set(0));
            // Drop in-flight accumulators too: the frames they belonged to are
            // being torn down, so they must not leak into the next frame.
            let _ = UNSAFE_IN_EXT_ACCUM.try_with(|d| d.set(0));
            let _ = UNSAFE_FRAME_EXT_ACCUM.try_with(|d| d.set(0));
            prior(info);
        }));
    });
}

#[no_mangle]
#[inline(always)]
pub extern "C" fn cpu_cycle_start_measurement() -> u64 {
    // Note: we intentionally do NOT check IN_EXTERNAL here.
    // Unsafe blocks must always be counted. The other direction
    // (external_call_start skips when IN_UNSAFE > 0) prevents
    // double-counting external calls made from within unsafe code.

    // Increment depth counter and check if we were already in an unsafe block
    let was_nested = IN_UNSAFE.with(|depth| {
        let current = depth.get();
        depth.set(current + 1);
        current > 0  // true if we were already in an unsafe block
    });

    if was_nested {
        return 0; // Skip nested unsafe blocks
    }

    // Only reach here for the outermost unsafe block.
    // Fix A: open a fresh per-frame external accumulator. Reset before the slot
    // lookup so the frame starts clean even if tracking init fails below.
    UNSAFE_FRAME_EXT_ACCUM.with(|acc| acc.set(0));

    let slot = match THREAD_SLOT.with(|s| s.get()).or_else(initialize_thread) {
        Some(slot) => slot,
        None => return 0,
    };

    let stats = &REGISTRY.threads[slot];
    stats.unsafe_blocks.fetch_add(1, Ordering::Relaxed);

    read_tsc_start()
}

#[no_mangle]
#[inline(always)]
pub extern "C" fn cpu_cycle_end_measurement(start_tsc: u64) {
    // Decrement depth counter and check if we're exiting the outermost unsafe block
    let is_outermost = IN_UNSAFE.with(|depth| {
        let current = depth.get();
        if current > 0 {
            depth.set(current - 1);
            current == 1  // true if we're exiting the outermost unsafe block (1 -> 0)
        } else {
            false
        }
    });

    if start_tsc == 0 || !is_outermost {
        return; // Was nested, in external, or not initialized
    }

    let slot = match THREAD_SLOT.with(|s| s.get()) {
        Some(slot) => slot,
        None => return,
    };

    let end_tsc = read_tsc_end();
    if end_tsc > start_tsc {
        let cycles = end_tsc - start_tsc;
        let stats = &REGISTRY.threads[slot];

        // Fix A: commit the parent FIRST so any concurrent stats snapshot always
        // observes unsafe_total >= unsafe_external.
        stats.unsafe_cycles.fetch_add(cycles, Ordering::Relaxed);

        // Carve the unsafe-external sub-interval out of the SAME bracket and clamp
        // it to this frame's own delta, so unsafe_external can never exceed
        // unsafe_total. The accumulator was filled by unsafe_external_call_end
        // while this frame was open; drain it here.
        let frame_ext = UNSAFE_FRAME_EXT_ACCUM.with(|acc| {
            let v = acc.get();
            acc.set(0);
            v
        });
        let ext = frame_ext.min(cycles);
        if ext > 0 {
            stats.unsafe_external_cycles.fetch_add(ext, Ordering::Relaxed);
        }

        // If we're inside an external call (callback scenario), record the overlap
        // so external_call_end can subtract it to avoid double-counting.
        let in_ext = IN_EXTERNAL.with(|d| d.get());
        if in_ext > 0 {
            UNSAFE_IN_EXT_ACCUM.with(|acc| acc.set(acc.get() + cycles));
        }
    } else {
        // TSC did not advance (skew / wrap): drop the frame accumulator so it
        // cannot leak into a later frame on this thread.
        UNSAFE_FRAME_EXT_ACCUM.with(|acc| acc.set(0));
    }
}

#[no_mangle]
#[inline(always)]
pub extern "C" fn external_call_start() -> u64 {
    // Only track external calls from safe code
    let in_unsafe = IN_UNSAFE.with(|depth| depth.get());
    if in_unsafe > 0 {
        return 0; // Skip, this is part of unsafe time
    }

    // Increment depth counter and check if we were already in an external call
    let was_nested = IN_EXTERNAL.with(|depth| {
        let current = depth.get();
        depth.set(current + 1);
        current > 0  // true if we were already in an external call
    });

    if was_nested {
        return 0; // Skip nested external calls
    }

    // Only reach here for the outermost external call
    let slot = match THREAD_SLOT.with(|s| s.get()).or_else(initialize_thread) {
        Some(slot) => slot,
        None => return 0,
    };

    let stats = &REGISTRY.threads[slot];
    stats.external_calls.fetch_add(1, Ordering::Relaxed);

    read_tsc_start()
}

#[no_mangle]
#[inline(always)]
pub extern "C" fn external_call_end(start_tsc: u64) {
    // Decrement depth counter and check if we're exiting the outermost call
    let is_outermost = IN_EXTERNAL.with(|depth| {
        let current = depth.get();
        if current > 0 {
            depth.set(current - 1);
            current == 1  // true if we're exiting the outermost call (1 -> 0)
        } else {
            false
        }
    });

    if start_tsc == 0 || !is_outermost {
        return; // Was nested, in unsafe, or not initialized
    }

    let slot = match THREAD_SLOT.with(|s| s.get()) {
        Some(slot) => slot,
        None => return,
    };

    let end_tsc = read_tsc_end();
    if end_tsc > start_tsc {
        let total_ext_cycles = end_tsc - start_tsc;

        // Subtract any unsafe cycles that occurred inside this external call
        // (callback scenario: e.g., qsort calling a comparator with unsafe code).
        // Those cycles are already counted as unsafe_cycles — don't also count as external.
        let overlap = UNSAFE_IN_EXT_ACCUM.with(|acc| {
            let v = acc.get();
            acc.set(0);
            v
        });

        let ext_only = total_ext_cycles.saturating_sub(overlap);
        if ext_only > 0 {
            let stats = &REGISTRY.threads[slot];
            stats.external_cycles.fetch_add(ext_only, Ordering::Relaxed);
        }
    } else {
        // Even if end_tsc <= start_tsc (TSC wrap), reset the accumulator
        UNSAFE_IN_EXT_ACCUM.with(|acc| acc.set(0));
    }
}

// ==========================================================================================
// === Unsafe-context external call hooks (Phase 6.5)
// ==========================================================================================
// These hooks are inserted by ExternalCallTracker around external calls that
// lie INSIDE an unsafe SESE region. Their TSC interval is a subset of the
// surrounding unsafe block's TSC interval, so the cycles they observe are
// already part of unsafe_cycles.
//
// Fix A: unsafe_external_call_end no longer commits to the global
// unsafe_external_cycles counter directly. It adds into UNSAFE_FRAME_EXT_ACCUM,
// a per-thread cell scoped to the current outermost unsafe frame. The frame's
// enclosing cpu_cycle_end_measurement commits it (clamped to the frame's own
// delta, parent committed first) when the outermost unsafe block closes. This
// makes unsafe_external <= unsafe_total structurally true even when a stats
// snapshot lands while a worker thread is parked inside an open unsafe block.
//
// These hooks must NOT touch IN_UNSAFE, IN_EXTERNAL, or UNSAFE_IN_EXT_ACCUM
// — they run in parallel with cpu_cycle_start/end_measurement and
// external_call_start/end and would corrupt the callback-subtraction logic
// if they did.

#[no_mangle]
#[inline(always)]
pub extern "C" fn unsafe_external_call_start() -> u64 {
    if IN_UNSAFE.with(|d| d.get()) == 0 {
        return 0;
    }

    let was_nested = IN_UNSAFE_EXT.with(|d| {
        let c = d.get();
        d.set(c + 1);
        c > 0
    });
    if was_nested {
        return 0;
    }

    let slot = match THREAD_SLOT.with(|s| s.get()).or_else(initialize_thread) {
        Some(slot) => slot,
        None => return 0,
    };
    REGISTRY.threads[slot]
        .unsafe_external_calls
        .fetch_add(1, Ordering::Relaxed);
    read_tsc_start()
}

#[no_mangle]
#[inline(always)]
pub extern "C" fn unsafe_external_call_end(start_tsc: u64) {
    let is_outermost = IN_UNSAFE_EXT.with(|d| {
        let c = d.get();
        if c > 0 {
            d.set(c - 1);
            c == 1
        } else {
            false
        }
    });

    if start_tsc == 0 || !is_outermost {
        return;
    }

    let end_tsc = read_tsc_end();
    if end_tsc > start_tsc {
        let cycles = end_tsc - start_tsc;
        // Fix A: accumulate into the current outermost unsafe frame instead of
        // committing to the global counter. The enclosing unsafe block commits it
        // (clamped to the frame delta) at frame close in cpu_cycle_end_measurement,
        // guaranteeing unsafe_external <= unsafe_total. No slot lookup is needed
        // here — the accumulator is thread-local.
        UNSAFE_FRAME_EXT_ACCUM.with(|acc| acc.set(acc.get() + cycles));
    }
}

// ==========================================================================================
// === Automatic Thread Lifecycle Management via pthread_create Interposition
// ==========================================================================================

type ThreadStartRoutine = extern "C" fn(*mut c_void) -> *mut c_void;

struct ThreadInfo {
    routine: ThreadStartRoutine,
    arg: *mut c_void,
}

extern "C" fn thread_start_wrapper(arg: *mut c_void) -> *mut c_void {
    // Automatic initialization
    initialize_thread();
    let info = unsafe { Box::from_raw(arg as *mut ThreadInfo) };
    let result = (info.routine)(info.arg);
    // Automatic cleanup
    thread_cleanup();
    result
}

type PthreadCreateFn = extern "C" fn(*mut libc::pthread_t, *const libc::pthread_attr_t, ThreadStartRoutine, *mut c_void) -> c_int;

lazy_static! {
    static ref REAL_PTHREAD_CREATE: Option<PthreadCreateFn> = {
        unsafe {
            // Try to find pthread_create using RTLD_NEXT first
            let symbol = libc::dlsym(libc::RTLD_NEXT, "pthread_create\0".as_ptr() as *const c_char);

            if !symbol.is_null() {
                Some(std::mem::transmute(symbol))
            } else {
                // If RTLD_NEXT fails, we'll disable interposition
                eprintln!("[Runtime] Warning: pthread_create interposition disabled - could not find symbol");
                None
            }
        }
    };
}

#[no_mangle]
pub extern "C" fn pthread_create(thread: *mut libc::pthread_t, attr: *const libc::pthread_attr_t, start_routine: ThreadStartRoutine, arg: *mut c_void) -> c_int {
    if let Some(real_pthread_create) = *REAL_PTHREAD_CREATE {
        let info = Box::new(ThreadInfo { routine: start_routine, arg });
        let info_ptr = Box::into_raw(info) as *mut c_void;
        real_pthread_create(thread, attr, thread_start_wrapper, info_ptr)
    } else {
        // Fallback: if interposition is disabled, return an error
        eprintln!("[Runtime] Warning: pthread_create called but interposition is disabled - returning error");
        libc::ENOSYS
    }
}

// ==========================================================================================
// === Statistics Reporting
// ==========================================================================================

#[no_mangle]
pub extern "C" fn print_cpu_cycle_stats() {
    // Use compare_exchange for exactly-once execution.
    if REGISTRY.stats_written.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        dump_stats();
    }
}

/// This function is registered to run when the program exits.
#[dtor]
fn final_cleanup() {
    print_cpu_cycle_stats();
}

struct Totals {
    total_cycles: u64,
    unsafe_cycles: u64,
    unsafe_external_cycles: u64,
    external_cycles: u64,
    unsafe_blocks: u64,
    external_calls: u64,
    unsafe_external_calls: u64,
}

fn calculate_total_stats() -> Totals {
    let mut t = Totals {
        total_cycles: 0,
        unsafe_cycles: 0,
        unsafe_external_cycles: 0,
        external_cycles: 0,
        unsafe_blocks: 0,
        external_calls: 0,
        unsafe_external_calls: 0,
    };

    let max_slot = REGISTRY.next_slot.load(Ordering::Acquire);
    for slot in 0..max_slot.min(MAX_THREADS) {
        let stats = &REGISTRY.threads[slot];

        let state = stats.state.load(Ordering::Acquire);
        if state == ThreadState::Uninitialized as usize {
            continue;
        }

        let thread_unsafe = stats.unsafe_cycles.load(Ordering::Acquire);
        let thread_unsafe_ext = stats.unsafe_external_cycles.load(Ordering::Acquire);
        let thread_external = stats.external_cycles.load(Ordering::Acquire);
        let thread_unsafe_blocks = stats.unsafe_blocks.load(Ordering::Acquire);
        let thread_external_calls = stats.external_calls.load(Ordering::Acquire);
        let thread_unsafe_ext_calls = stats.unsafe_external_calls.load(Ordering::Acquire);

        let thread_total = if state == ThreadState::Active as usize {
            let current_tsc = read_tsc_end();
            let start_tsc = stats.start_tsc.load(Ordering::Acquire);
            if current_tsc > start_tsc {
                current_tsc - start_tsc
            } else {
                0
            }
        } else {
            stats.total_cycles.load(Ordering::Acquire)
        };

        t.total_cycles += thread_total;
        t.unsafe_cycles += thread_unsafe;
        t.unsafe_external_cycles += thread_unsafe_ext;
        t.external_cycles += thread_external;
        t.unsafe_blocks += thread_unsafe_blocks;
        t.external_calls += thread_external_calls;
        t.unsafe_external_calls += thread_unsafe_ext_calls;
    }

    t
}

fn dump_stats() {
    let t = calculate_total_stats();

    // Raw counters only. Derived quantities (internal_cycles = total - external,
    // unsafe_cycles_internal = unsafe_total - unsafe_external, percentages, etc.)
    // are the digester's job. Emitting them here would bake assumptions about
    // the invariants into the on-disk format, and silent clamping (saturating_sub)
    // would hide runtime/pass bugs rather than surface them.
    let stats_json = format!(
        concat!(
            "{{",
            "\"total_cycles\":{},",
            "\"unsafe_cycles_total\":{},",
            "\"unsafe_cycles_external\":{},",
            "\"external_cycles\":{},",
            "\"unsafe_blocks\":{},",
            "\"external_calls\":{},",
            "\"unsafe_external_calls\":{}",
            "}}"
        ),
        t.total_cycles,
        t.unsafe_cycles,
        t.unsafe_external_cycles,
        t.external_cycles,
        t.unsafe_blocks,
        t.external_calls,
        t.unsafe_external_calls,
    );

    let _ = write_stat_json("cpu_cycle", &stats_json);
}
