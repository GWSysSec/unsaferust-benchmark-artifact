//! Unsafe-oriented test for `log`.
//!
//! `log` reads 0 unsafe instructions in the corpus because its only executable
//! unsafe lives in `set_logger_racy`, whose body writes the `static mut LOGGER`
//! (`LOGGER = logger;` at src/lib.rs:458 — an unsafe store to a mutable static).
//! The existing suite never reaches it: the global logger is set at most once
//! per process, so tests avoid initializing it. This dedicated test binary
//! initializes the logger via the racy API on a fresh process, exercising that
//! unsafe store.
use log::{LevelFilter, Metadata, Record};

struct ProbeLogger;

impl log::Log for ProbeLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        // Touch the record so the installed `&dyn Log` is genuinely used and
        // the call cannot be optimized away.
        let _ = format!("{}: {}", record.level(), record.args());
    }
    fn flush(&self) {}
}

static LOGGER: ProbeLogger = ProbeLogger;

#[test]
fn set_logger_racy_writes_static_mut_logger() {
    // SAFETY: this is the sole logger-initialization call in this test binary,
    // running on its own fresh process, so nothing races with the static-mut
    // write inside `set_logger_racy`.
    unsafe {
        log::set_logger_racy(&LOGGER).expect("racy logger init must succeed once");
        log::set_max_level_racy(LevelFilter::Trace);
    }

    // Drive the now-installed logger across several levels.
    log::error!("probe error {}", 1);
    log::warn!("probe warn {}", 2);
    log::info!("probe info {}", 3);
    log::debug!("probe debug {}", 4);
    log::trace!("probe trace {}", 5);

    assert_eq!(log::max_level(), LevelFilter::Trace);
}
