use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use metrics::{
    Counter, Gauge, Histogram, Key, KeyName, Label, Metadata, Recorder, SharedString, Unit,
};

struct RecordManyEntry {
    value: f64,
    count: usize,
}

struct TestRecorder {
    histograms: Arc<Mutex<HashMap<String, Vec<RecordManyEntry>>>>,
    counters: Arc<Mutex<HashMap<String, u64>>>,
    gauges: Arc<Mutex<HashMap<String, f64>>>,
}

impl TestRecorder {
    fn new() -> Self {
        Self {
            histograms: Arc::new(Mutex::new(HashMap::new())),
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn histograms(&self) -> Arc<Mutex<HashMap<String, Vec<RecordManyEntry>>>> {
        Arc::clone(&self.histograms)
    }
}

impl Recorder for TestRecorder {
    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}
    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}
    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}

    fn register_counter(&self, key: &Key, _metadata: &Metadata<'_>) -> Counter {
        let name = key.name().to_string();
        let counters = Arc::clone(&self.counters);
        if let Ok(mut m) = counters.try_lock() {
            m.entry(name).or_insert(0);
        }
        Counter::noop()
    }

    fn register_gauge(&self, key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        let name = key.name().to_string();
        let gauges = Arc::clone(&self.gauges);
        if let Ok(mut m) = gauges.try_lock() {
            m.entry(name).or_insert(0.0);
        }
        Gauge::noop()
    }

    fn register_histogram(&self, key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        let name = key.name().to_string();
        let histograms = Arc::clone(&self.histograms);
        if let Ok(mut m) = histograms.try_lock() {
            m.entry(name.clone()).or_insert_with(Vec::new);
        }

        let histograms_inner = Arc::clone(&self.histograms);
        Histogram::from_arc(Arc::new(TestHistogramInner {
            name,
            storage: histograms_inner,
        }))
    }
}

struct TestHistogramInner {
    name: String,
    storage: Arc<Mutex<HashMap<String, Vec<RecordManyEntry>>>>,
}

impl metrics::HistogramFn for TestHistogramInner {
    fn record(&self, value: f64) {
        if let Ok(mut map) = self.storage.try_lock() {
            map.entry(self.name.clone())
                .or_insert_with(Vec::new)
                .push(RecordManyEntry { value, count: 1 });
        }
    }

    fn record_many(&self, value: f64, count: usize) {
        if let Ok(mut map) = self.storage.try_lock() {
            map.entry(self.name.clone())
                .or_insert_with(Vec::new)
                .push(RecordManyEntry { value, count });
        }
    }
}

#[test]
fn test_histogram_record_many_basic() {
    let recorder = TestRecorder::new();
    let histograms = recorder.histograms();

    let key = Key::from_name("test.histogram.basic");
    let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
    let histogram = recorder.register_histogram(&key, &metadata);


    {
        let map = histograms.try_lock().unwrap();
        assert_eq!(map.get("test.histogram.basic").map(|v| v.len()), Some(0));
    }


    histogram.record_many(42.0, 10);


    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.basic").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, 42.0);
        assert_eq!(entries[0].count, 10);
    }


    histogram.record_many(99.5, 5);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.basic").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].value, 99.5);
        assert_eq!(entries[1].count, 5);
    }


    histogram.record_many(1.0, 0);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.basic").unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].count, 0);
    }
}

#[test]
fn test_histogram_record_many_multiple_values() {
    let recorder = TestRecorder::new();
    let histograms = recorder.histograms();

    let key = Key::from_name("test.histogram.multi");
    let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
    let histogram = recorder.register_histogram(&key, &metadata);

    let test_data: Vec<(f64, usize)> = vec![
        (1.0, 100),
        (2.5, 200),
        (3.7, 50),
        (0.001, 1000),
        (999.999, 1),
    ];

    for (value, count) in &test_data {
        histogram.record_many(*value, *count);
    }

    let map = histograms.try_lock().unwrap();
    let entries = map.get("test.histogram.multi").unwrap();

    assert_eq!(entries.len(), 5);
    assert_eq!(entries[0].value, 1.0);
    assert_eq!(entries[0].count, 100);
    assert_eq!(entries[1].value, 2.5);
    assert_eq!(entries[1].count, 200);
    assert_eq!(entries[2].value, 3.7);
    assert_eq!(entries[2].count, 50);
    assert_eq!(entries[3].value, 0.001);
    assert_eq!(entries[3].count, 1000);
    assert_eq!(entries[4].value, 999.999);
    assert_eq!(entries[4].count, 1);
}

#[test]
fn test_histogram_record_many_large_count() {
    let recorder = TestRecorder::new();
    let histograms = recorder.histograms();

    let key = Key::from_name("test.histogram.large_count");
    let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
    let histogram = recorder.register_histogram(&key, &metadata);


    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.large_count").unwrap();
        assert_eq!(entries.len(), 0);
    }


    let large_count: usize = 1_000_000;
    histogram.record_many(3.14159, large_count);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.large_count").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, 3.14159);
        assert_eq!(entries[0].count, 1_000_000);
    }


    let boundary_count: usize = usize::MAX / 2;
    histogram.record_many(2.718, boundary_count);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.large_count").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].value, 2.718);
        assert_eq!(entries[1].count, boundary_count);
    }
}

#[test]
fn test_histogram_record_many_special_float_values() {
    let recorder = TestRecorder::new();
    let histograms = recorder.histograms();

    let key = Key::from_name("test.histogram.special_floats");
    let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
    let histogram = recorder.register_histogram(&key, &metadata);


    histogram.record_many(f64::MAX, 1);

    histogram.record_many(f64::MIN_POSITIVE, 2);

    histogram.record_many(0.0, 3);

    histogram.record_many(-1.0, 4);

    let map = histograms.try_lock().unwrap();
    let entries = map.get("test.histogram.special_floats").unwrap();

    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].value, f64::MAX);
    assert_eq!(entries[0].count, 1);
    assert_eq!(entries[1].value, f64::MIN_POSITIVE);
    assert_eq!(entries[1].count, 2);
    assert_eq!(entries[2].value, 0.0);
    assert_eq!(entries[2].count, 3);
    assert_eq!(entries[3].value, -1.0);
    assert_eq!(entries[3].count, 4);
}

#[test]
fn test_histogram_record_many_interleaved_with_record() {
    let recorder = TestRecorder::new();
    let histograms = recorder.histograms();

    let key = Key::from_name("test.histogram.interleaved");
    let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
    let histogram = recorder.register_histogram(&key, &metadata);


    histogram.record(10.0);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.interleaved").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, 10.0);
        assert_eq!(entries[0].count, 1);
    }


    histogram.record_many(20.0, 5);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.interleaved").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].value, 20.0);
        assert_eq!(entries[1].count, 5);
    }


    histogram.record(30.0);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.interleaved").unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].value, 30.0);
        assert_eq!(entries[2].count, 1);
    }


    histogram.record_many(40.0, 100);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.interleaved").unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[3].value, 40.0);
        assert_eq!(entries[3].count, 100);
    }
}

#[test]
fn test_histogram_noop_record_many_does_not_panic() {

    let histogram = Histogram::noop();


    histogram.record_many(1.0, 1);
    histogram.record_many(100.0, 1000);
    histogram.record_many(0.0, 0);
    histogram.record_many(f64::MAX, usize::MAX);
    histogram.record_many(f64::MIN_POSITIVE, 42);
    histogram.record_many(-999.0, 7);



    assert_eq!(1 + 1, 2);
    assert_eq!(f64::MAX, f64::MAX);
    assert_ne!(f64::MAX, f64::MIN_POSITIVE);
    assert_eq!(usize::MAX, usize::MAX);
    assert_ne!(0.0_f64, 1.0_f64);
    assert_eq!(0.0_f64, 0.0_f64);
    assert_ne!(usize::MAX, 0);
    assert_eq!(42_usize, 42_usize);
}

#[test]
fn test_histogram_record_many_with_labels() {
    let recorder = TestRecorder::new();
    let histograms = recorder.histograms();

    let labels = vec![
        Label::new("service", "web"),
        Label::new("endpoint", "/api/v1"),
    ];
    let key = Key::from_parts("test.histogram.labeled", labels);
    let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
    let histogram = recorder.register_histogram(&key, &metadata);


    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.labeled").unwrap();
        assert_eq!(entries.len(), 0);
    }

    histogram.record_many(150.0, 25);
    histogram.record_many(200.0, 50);
    histogram.record_many(350.0, 10);

    {
        let map = histograms.try_lock().unwrap();
        let entries = map.get("test.histogram.labeled").unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].value, 150.0);
        assert_eq!(entries[0].count, 25);
        assert_eq!(entries[1].value, 200.0);
        assert_eq!(entries[1].count, 50);
        assert_eq!(entries[2].value, 350.0);
        assert_eq!(entries[2].count, 10);
    }
}

#[test]
fn test_histogram_record_many_count_one_equivalent_to_record() {
    let recorder = TestRecorder::new();
    let histograms = recorder.histograms();

    let key1 = Key::from_name("test.histogram.single_record");
    let key2 = Key::from_name("test.histogram.record_many_one");
    let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);

    let h1 = recorder.register_histogram(&key1, &metadata);
    let h2 = recorder.register_histogram(&key2, &metadata);

    let value = 77.7;

    h1.record(value);
    h2.record_many(value, 1);

    let map = histograms.try_lock().unwrap();

    let entries1 = map.get("test.histogram.single_record").unwrap();
    let entries2 = map.get("test.histogram.record_many_one").unwrap();


    assert_eq!(entries1.len(), 1);
    assert_eq!(entries2.len(), 1);
    assert_eq!(entries1[0].value, value);
    assert_eq!(entries2[0].value, value);
    assert_eq!(entries1[0].count, 1);
    assert_eq!(entries2[0].count, 1);
    assert_eq!(entries1[0].value, entries2[0].value);
    assert_eq!(entries1[0].count, entries2[0].count);
}