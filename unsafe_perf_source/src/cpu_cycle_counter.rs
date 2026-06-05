//! Runtime library for CPU cycle tracking.
//! This version uses pthread_create interposition for fully automatic
//! and accurate tracking of the entire lifecycle of every thread, and
//! it deducts time spent in external library calls.

use ctor::dtor;
use lazy_static::lazy_static;
use std::cell::Cell;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use crate::write_output;

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
    last_known_tsc: AtomicU64,

    // Time spent in each mutually exclusive state
    normal_cycles: AtomicU64,
    unsafe_cycles: AtomicU64,
    external_safe_cycles: AtomicU64,
    external_unsafe_cycles: AtomicU64,

    // Block counts
    unsafe_blocks: AtomicU64,
    external_calls: AtomicU64,

    _padding: [u64; 2],
}

impl ThreadStats {
    const fn new() -> Self {
        Self {
            thread_id: AtomicU64::new(0),
            state: AtomicUsize::new(ThreadState::Uninitialized as usize),
            start_tsc: AtomicU64::new(0),
            last_known_tsc: AtomicU64::new(0),
            normal_cycles: AtomicU64::new(0),
            unsafe_cycles: AtomicU64::new(0),
            external_safe_cycles: AtomicU64::new(0),
            external_unsafe_cycles: AtomicU64::new(0),
            unsafe_blocks: AtomicU64::new(0),
            external_calls: AtomicU64::new(0),
            _padding: [0; 2],
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
                    stats.last_known_tsc.store(0, Ordering::Relaxed);
                    stats.normal_cycles.store(0, Ordering::Relaxed);
                    stats.unsafe_cycles.store(0, Ordering::Relaxed);
                    stats.external_safe_cycles.store(0, Ordering::Relaxed);
                    stats.external_unsafe_cycles.store(0, Ordering::Relaxed);
                    stats.unsafe_blocks.store(0, Ordering::Relaxed);
                    stats.external_calls.store(0, Ordering::Relaxed);
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

// Fixed: Use atomic operations and proper nesting context
const MAX_CONTEXT_DEPTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum ExecutionState {
    Normal = 0,           // Safe Rust code
    Unsafe = 1,           // Unsafe Rust code (not in external call)
    ExternalSafe = 2,     // External call from safe code
    ExternalUnsafe = 3,   // External call from unsafe code
}

#[derive(Debug, Clone, Copy)]
struct ContextFrame {
    state: ExecutionState,
    start_tsc: u64,
}

thread_local! {
    static THREAD_SLOT: Cell<Option<usize>> = Cell::new(None);
    static CONTEXT_STACK: Cell<[ContextFrame; MAX_CONTEXT_DEPTH]> = Cell::new([ContextFrame { state: ExecutionState::Normal, start_tsc: 0 }; MAX_CONTEXT_DEPTH]);
    static STACK_DEPTH: Cell<usize> = Cell::new(0);
}

#[inline(always)]
fn read_tsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
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
            let tsc = read_tsc();
            let stats = &REGISTRY.threads[slot];
            stats.thread_id.store(get_thread_id(), Ordering::Relaxed);
            stats.start_tsc.store(tsc, Ordering::Relaxed);
            stats.last_known_tsc.store(tsc, Ordering::Release);
            stats.state.store(ThreadState::Active as usize, Ordering::Release);

            // Initialize context stack with Normal state
            CONTEXT_STACK.with(|stack_cell| {
                let mut stack = stack_cell.get();
                stack[0] = ContextFrame { state: ExecutionState::Normal, start_tsc: tsc };
                stack_cell.set(stack);
            });
            STACK_DEPTH.with(|depth_cell| depth_cell.set(1));

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

/// Atomic state transition with cycle accounting
fn transition_state(new_state: ExecutionState) -> Result<(), &'static str> {
    let slot = match THREAD_SLOT.with(|s| s.get()).or_else(initialize_thread) {
        Some(slot) => slot,
        None => return Err("Thread not initialized"),
    };

    let current_tsc = read_tsc();
    let stats = &REGISTRY.threads[slot];

    CONTEXT_STACK.with(|stack_cell| {
        STACK_DEPTH.with(|depth_cell| {
            let depth = depth_cell.get();
            if depth == 0 || depth > MAX_CONTEXT_DEPTH {
                return Err("Invalid stack depth");
            }

            let mut stack = stack_cell.get();
            let current_frame = &mut stack[depth - 1];

            // Account for time spent in current state
            if current_frame.start_tsc > 0 && current_tsc > current_frame.start_tsc {
                let duration = current_tsc - current_frame.start_tsc;

                match current_frame.state {
                    ExecutionState::Normal => stats.normal_cycles.fetch_add(duration, Ordering::Relaxed),
                    ExecutionState::Unsafe => stats.unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                    ExecutionState::ExternalSafe => stats.external_safe_cycles.fetch_add(duration, Ordering::Relaxed),
                    ExecutionState::ExternalUnsafe => stats.external_unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                };
            }

            // Update current frame to new state
            current_frame.state = new_state;
            current_frame.start_tsc = current_tsc;

            stack_cell.set(stack);
            Ok(())
        })
    })
}

/// Push new context onto stack (for external calls)
fn push_context(new_state: ExecutionState) -> Result<(), &'static str> {
    let slot = match THREAD_SLOT.with(|s| s.get()).or_else(initialize_thread) {
        Some(slot) => slot,
        None => return Err("Thread not initialized"),
    };

    let current_tsc = read_tsc();
    let stats = &REGISTRY.threads[slot];

    CONTEXT_STACK.with(|stack_cell| {
        STACK_DEPTH.with(|depth_cell| {
            let depth = depth_cell.get();
            if depth >= MAX_CONTEXT_DEPTH {
                return Err("Context stack overflow");
            }

            let mut stack = stack_cell.get();

            // Account for time in current state
            if depth > 0 {
                let current_frame = &mut stack[depth - 1];
                if current_frame.start_tsc > 0 && current_tsc > current_frame.start_tsc {
                    let duration = current_tsc - current_frame.start_tsc;

                    match current_frame.state {
                        ExecutionState::Normal => stats.normal_cycles.fetch_add(duration, Ordering::Relaxed),
                        ExecutionState::Unsafe => stats.unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                        ExecutionState::ExternalSafe => stats.external_safe_cycles.fetch_add(duration, Ordering::Relaxed),
                        ExecutionState::ExternalUnsafe => stats.external_unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                    };
                }
            }

            // Push new context
            stack[depth] = ContextFrame { state: new_state, start_tsc: current_tsc };
            depth_cell.set(depth + 1);
            stack_cell.set(stack);
            Ok(())
        })
    })
}

/// Pop context from stack (for external call returns)
fn pop_context() -> Result<(), &'static str> {
    let slot = match THREAD_SLOT.with(|s| s.get()) {
        Some(slot) => slot,
        None => return Err("Thread not initialized"),
    };

    let current_tsc = read_tsc();
    let stats = &REGISTRY.threads[slot];

    CONTEXT_STACK.with(|stack_cell| {
        STACK_DEPTH.with(|depth_cell| {
            let depth = depth_cell.get();
            if depth <= 1 {
                return Err("Cannot pop from empty context stack");
            }

            let mut stack = stack_cell.get();
            let current_frame = &mut stack[depth - 1];

            // Account for time in current state
            if current_frame.start_tsc > 0 && current_tsc > current_frame.start_tsc {
                let duration = current_tsc - current_frame.start_tsc;

                match current_frame.state {
                    ExecutionState::Normal => stats.normal_cycles.fetch_add(duration, Ordering::Relaxed),
                    ExecutionState::Unsafe => stats.unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                    ExecutionState::ExternalSafe => stats.external_safe_cycles.fetch_add(duration, Ordering::Relaxed),
                    ExecutionState::ExternalUnsafe => stats.external_unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                };
            }

            // Pop context and resume previous state
            depth_cell.set(depth - 1);
            let previous_frame = &mut stack[depth - 2];
            previous_frame.start_tsc = current_tsc; // Reset timing for resumed context

            stack_cell.set(stack);
            Ok(())
        })
    })
}

/// Marks the current thread as terminated and records its final timestamp.
fn thread_cleanup() {
    if let Some(slot) = THREAD_SLOT.with(|s| s.get()) {
        if slot < MAX_THREADS {
            let final_tsc = read_tsc();
            let stats = &REGISTRY.threads[slot];

            // Account for any remaining time in current state
            CONTEXT_STACK.with(|stack_cell| {
                STACK_DEPTH.with(|depth_cell| {
                    let depth = depth_cell.get();
                    if depth > 0 {
                        let stack = stack_cell.get();
                        let current_frame = &stack[depth - 1];

                        if current_frame.start_tsc > 0 && final_tsc > current_frame.start_tsc {
                            let duration = final_tsc - current_frame.start_tsc;

                            match current_frame.state {
                                ExecutionState::Normal => stats.normal_cycles.fetch_add(duration, Ordering::Relaxed),
                                ExecutionState::Unsafe => stats.unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                                ExecutionState::ExternalSafe => stats.external_safe_cycles.fetch_add(duration, Ordering::Relaxed),
                                ExecutionState::ExternalUnsafe => stats.external_unsafe_cycles.fetch_add(duration, Ordering::Relaxed),
                            };
                        }
                    }
                });
            });

            stats.last_known_tsc.store(final_tsc, Ordering::Release);
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

#[no_mangle]
#[inline(always)]
pub extern "C" fn cpu_cycle_start_measurement() -> u64 {
    let slot = match THREAD_SLOT.with(|s| s.get()).or_else(initialize_thread) {
        Some(slot) => slot,
        None => return 0,
    };

    // Transition from current state to Unsafe
    let current_state = CONTEXT_STACK.with(|stack_cell| {
        STACK_DEPTH.with(|depth_cell| {
            let depth = depth_cell.get();
            if depth > 0 {
                let stack = stack_cell.get();
                stack[depth - 1].state
            } else {
                ExecutionState::Normal
            }
        })
    });

    let new_state = match current_state {
        ExecutionState::Normal => ExecutionState::Unsafe,
        ExecutionState::ExternalSafe => ExecutionState::ExternalUnsafe,
        _ => current_state, // Already in unsafe or external_unsafe
    };

    if transition_state(new_state).is_ok() {
        let stats = &REGISTRY.threads[slot];
        stats.unsafe_blocks.fetch_add(1, Ordering::Relaxed);
    }

    read_tsc()
}

#[no_mangle]
#[inline(always)]
pub extern "C" fn cpu_cycle_end_measurement(start_tsc: u64) {
    let _start_tsc = start_tsc; // Parameter for compatibility, but we use state machine timing

    // Transition from unsafe state back to safe state
    let current_state = CONTEXT_STACK.with(|stack_cell| {
        STACK_DEPTH.with(|depth_cell| {
            let depth = depth_cell.get();
            if depth > 0 {
                let stack = stack_cell.get();
                stack[depth - 1].state
            } else {
                ExecutionState::Normal
            }
        })
    });

    let new_state = match current_state {
        ExecutionState::Unsafe => ExecutionState::Normal,
        ExecutionState::ExternalUnsafe => ExecutionState::ExternalSafe,
        _ => current_state, // Not in unsafe state
    };

    let _ = transition_state(new_state);
}

#[no_mangle]
pub extern "C" fn external_call_start() -> u64 {
    let slot = match THREAD_SLOT.with(|s| s.get()).or_else(initialize_thread) {
        Some(slot) => slot,
        None => return 0,
    };

    // Determine external call state based on current context
    let new_state = CONTEXT_STACK.with(|stack_cell| {
        STACK_DEPTH.with(|depth_cell| {
            let depth = depth_cell.get();
            if depth > 0 {
                let stack = stack_cell.get();
                match stack[depth - 1].state {
                    ExecutionState::Normal => ExecutionState::ExternalSafe,
                    ExecutionState::Unsafe => ExecutionState::ExternalUnsafe,
                    _ => return ExecutionState::ExternalSafe, // Default
                }
            } else {
                ExecutionState::ExternalSafe
            }
        })
    });

    if push_context(new_state).is_ok() {
        let stats = &REGISTRY.threads[slot];
        stats.external_calls.fetch_add(1, Ordering::Relaxed);
    }

    read_tsc()
}

#[no_mangle]
pub extern "C" fn external_call_end(start_tsc: u64) {
    let _start_tsc = start_tsc; // Parameter for compatibility
    let _ = pop_context();
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
    // 1. Automatic Initialization
    initialize_thread();
    let info = unsafe { Box::from_raw(arg as *mut ThreadInfo) };
    let result = (info.routine)(info.arg);
    // 2. Automatic Cleanup
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
        // This prevents infinite recursion and crashes
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

fn calculate_total_stats() -> (u64, u64, u64, u64, u64, u64, u64) {
    let mut total_normal = 0;
    let mut total_unsafe = 0;
    let mut total_external_safe = 0;
    let mut total_external_unsafe = 0;
    let mut total_unsafe_blocks = 0;
    let mut total_external_calls = 0;

    let max_slot = REGISTRY.next_slot.load(Ordering::Acquire);
    for slot in 0..max_slot.min(MAX_THREADS) {
        let stats = &REGISTRY.threads[slot];
        let state = stats.state.load(Ordering::Acquire);
        if state == ThreadState::Uninitialized as usize {
            continue;
        }

        total_normal += stats.normal_cycles.load(Ordering::Acquire);
        total_unsafe += stats.unsafe_cycles.load(Ordering::Acquire);
        total_external_safe += stats.external_safe_cycles.load(Ordering::Acquire);
        total_external_unsafe += stats.external_unsafe_cycles.load(Ordering::Acquire);
        total_unsafe_blocks += stats.unsafe_blocks.load(Ordering::Acquire);
        total_external_calls += stats.external_calls.load(Ordering::Acquire);
    }

    let total_program_cycles = total_normal + total_unsafe + total_external_safe + total_external_unsafe;

    (total_program_cycles, total_normal, total_unsafe, total_external_safe, total_external_unsafe, total_unsafe_blocks, total_external_calls)
}

fn dump_stats() {
    let (total_cycles, normal_cycles, unsafe_cycles, external_safe_cycles, external_unsafe_cycles, _unsafe_blocks, _external_calls) = calculate_total_stats();

    // Clean accounting - no overlaps, no double counting
    let internal_cycles = normal_cycles + unsafe_cycles;
    let external_cycles = external_safe_cycles + external_unsafe_cycles;

    let unsafe_percentage = if internal_cycles > 0 {
        (unsafe_cycles as f64 / internal_cycles as f64) * 100.0
    } else {
        0.0
    };

    // Create structured output for script parsing
    let output = format!(
        concat!(
            "\n===== CPU Cycle Statistics =====\n",
            "Total cycles: {}\n",
            "Unsafe cycles: {}\n",
            "External cycles: {}\n",
            "Internal cycles: {}\n",
            "Unsafe percentage: {:.2}\n",
        ),
        total_cycles, unsafe_cycles, external_cycles, internal_cycles, unsafe_percentage
    );

    // Write structured output to file for script parsing
    let _ = write_output(&output, "cpu_cycle.stat");
}
