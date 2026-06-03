








use log::{LevelFilter, Metadata, Record};

struct ProbeLogger;

impl log::Log for ProbeLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {


        let _ = format!("{}: {}", record.level(), record.args());
    }
    fn flush(&self) {}
}

static LOGGER: ProbeLogger = ProbeLogger;

#[test]
fn set_logger_racy_writes_static_mut_logger() {



    unsafe {
        log::set_logger_racy(&LOGGER).expect("racy logger init must succeed once");
        log::set_max_level_racy(LevelFilter::Trace);
    }


    log::error!("probe error {}", 1);
    log::warn!("probe warn {}", 2);
    log::info!("probe info {}", 3);
    log::debug!("probe debug {}", 4);
    log::trace!("probe trace {}", 5);

    assert_eq!(log::max_level(), LevelFilter::Trace);
}
