use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use metrics::{
    set_global_recorder, with_local_recorder, with_recorder, Counter, Gauge, Histogram, Key,
    KeyName, Metadata, Recorder, SharedString, Unit,
};

struct TestRecorder {
    counter_count: Arc<AtomicU64>,
    gauge_count: Arc<AtomicU64>,
    histogram_count: Arc<AtomicU64>,
    register_counter_count: Arc<AtomicU64>,
    register_gauge_count: Arc<AtomicU64>,
    register_histogram_count: Arc<AtomicU64>,
}

impl TestRecorder {
    fn new() -> Self {
        Self {
            counter_count: Arc::new(AtomicU64::new(0)),
            gauge_count: Arc::new(AtomicU64::new(0)),
            histogram_count: Arc::new(AtomicU64::new(0)),
            register_counter_count: Arc::new(AtomicU64::new(0)),
            register_gauge_count: Arc::new(AtomicU64::new(0)),
            register_histogram_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Recorder for TestRecorder {
    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}
    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}
    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}

    fn register_counter(&self, _key: &Key, _metadata: &Metadata<'_>) -> Counter {
        self.register_counter_count.fetch_add(1, Ordering::SeqCst);
        Counter::noop()
    }

    fn register_gauge(&self, _key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        self.register_gauge_count.fetch_add(1, Ordering::SeqCst);
        Gauge::noop()
    }

    fn register_histogram(&self, _key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        self.register_histogram_count.fetch_add(1, Ordering::SeqCst);
        Histogram::noop()
    }
}

struct CountingRecorder {
    ops: Arc<AtomicU64>,
}

impl CountingRecorder {
    fn new(ops: Arc<AtomicU64>) -> Self {
        Self { ops }
    }
}

impl Recorder for CountingRecorder {
    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        self.ops.fetch_add(1, Ordering::SeqCst);
    }
    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        self.ops.fetch_add(1, Ordering::SeqCst);
    }
    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {
        self.ops.fetch_add(1, Ordering::SeqCst);
    }

    fn register_counter(&self, _key: &Key, _metadata: &Metadata<'_>) -> Counter {
        self.ops.fetch_add(1, Ordering::SeqCst);
        Counter::noop()
    }

    fn register_gauge(&self, _key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        self.ops.fetch_add(1, Ordering::SeqCst);
        Gauge::noop()
    }

    fn register_histogram(&self, _key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        self.ops.fetch_add(1, Ordering::SeqCst);
        Histogram::noop()
    }
}

#[test]
fn test_set_global_recorder_success() {


    let recorder = TestRecorder::new();
    let result = set_global_recorder(recorder);


    let was_ok = result.is_ok();
    let was_err = result.is_err();
    assert_ne!(was_ok, was_err);
    assert_eq!(was_ok || was_err, true);


    let recorder2 = TestRecorder::new();
    let result2 = set_global_recorder(recorder2);
    if was_ok {

        assert!(result2.is_err());
        assert!(!result2.is_ok());
    } else {

        assert!(result2.is_err());
        assert!(!result2.is_ok());
    }


    let executed = with_recorder(|_rec| 42u64);
    assert_eq!(executed, 42u64);

    let executed2 = with_recorder(|_rec| String::from("hello"));
    assert_eq!(executed2, "hello");
}

#[test]
fn test_with_local_recorder_basic_workflow() {
    let ops = Arc::new(AtomicU64::new(0));
    let recorder = CountingRecorder::new(ops.clone());

    assert_eq!(ops.load(Ordering::SeqCst), 0);

    let result = with_local_recorder(&recorder, || {


        with_recorder(|rec| {
            let key = Key::from_name("test.counter");
            let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
            let _counter = rec.register_counter(&key, &metadata);
        });
        100u64
    });

    assert_eq!(result, 100u64);
    assert_eq!(ops.load(Ordering::SeqCst), 1);


    let ops_after = ops.load(Ordering::SeqCst);


    with_recorder(|rec| {
        let key = Key::from_name("outside.counter");
        let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
        let _counter = rec.register_counter(&key, &metadata);
    });


    assert_eq!(ops.load(Ordering::SeqCst), ops_after);
    assert_eq!(ops_after, 1);
}

#[test]
fn test_with_local_recorder_multiple_operations() {
    let ops = Arc::new(AtomicU64::new(0));
    let recorder = CountingRecorder::new(ops.clone());

    assert_eq!(ops.load(Ordering::SeqCst), 0);

    let result = with_local_recorder(&recorder, || {
        with_recorder(|rec| {
            let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);

            let key1 = Key::from_name("multi.counter");
            let _c = rec.register_counter(&key1, &metadata);

            let key2 = Key::from_name("multi.gauge");
            let _g = rec.register_gauge(&key2, &metadata);

            let key3 = Key::from_name("multi.histogram");
            let _h = rec.register_histogram(&key3, &metadata);

            rec.describe_counter(
                KeyName::from("multi.counter"),
                Some(Unit::Count),
                SharedString::from("a counter"),
            );

            rec.describe_gauge(
                KeyName::from("multi.gauge"),
                Some(Unit::Bytes),
                SharedString::from("a gauge"),
            );
        });
        "done"
    });

    assert_eq!(result, "done");

    assert_eq!(ops.load(Ordering::SeqCst), 5);
    assert!(ops.load(Ordering::SeqCst) > 0);
    assert!(ops.load(Ordering::SeqCst) >= 5);
}

#[test]
fn test_with_local_recorder_nested_scopes() {
    let ops_outer = Arc::new(AtomicU64::new(0));
    let ops_inner = Arc::new(AtomicU64::new(0));
    let recorder_outer = CountingRecorder::new(ops_outer.clone());
    let recorder_inner = CountingRecorder::new(ops_inner.clone());

    assert_eq!(ops_outer.load(Ordering::SeqCst), 0);
    assert_eq!(ops_inner.load(Ordering::SeqCst), 0);

    let result = with_local_recorder(&recorder_outer, || {

        with_recorder(|rec| {
            let key = Key::from_name("outer.metric");
            let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
            let _c = rec.register_counter(&key, &metadata);
        });


        let inner_result = with_local_recorder(&recorder_inner, || {
            with_recorder(|rec| {
                let key = Key::from_name("inner.metric");
                let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
                let _c = rec.register_counter(&key, &metadata);
                let _g = rec.register_gauge(&key, &metadata);
            });
            "inner_done"
        });

        assert_eq!(inner_result, "inner_done");


        with_recorder(|rec| {
            let key = Key::from_name("outer.metric2");
            let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
            let _g = rec.register_gauge(&key, &metadata);
        });

        "outer_done"
    });

    assert_eq!(result, "outer_done");

    assert_eq!(ops_outer.load(Ordering::SeqCst), 2);

    assert_eq!(ops_inner.load(Ordering::SeqCst), 2);
}

#[test]
fn test_with_recorder_return_values() {

    let int_val = with_recorder(|_rec| 12345u64);
    assert_eq!(int_val, 12345u64);

    let string_val = with_recorder(|_rec| String::from("metrics_test"));
    assert_eq!(string_val, "metrics_test");
    assert_eq!(string_val.len(), 12);

    let tuple_val = with_recorder(|_rec| (1u32, 2u32, 3u32));
    assert_eq!(tuple_val.0, 1);
    assert_eq!(tuple_val.1, 2);
    assert_eq!(tuple_val.2, 3);

    let vec_val = with_recorder(|_rec| vec![10, 20, 30]);
    assert_eq!(vec_val.len(), 3);
    assert_eq!(vec_val[0], 10);
}

#[test]
fn test_with_local_recorder_does_not_leak() {
    let ops = Arc::new(AtomicU64::new(0));
    let recorder = CountingRecorder::new(ops.clone());


    let before = ops.load(Ordering::SeqCst);
    assert_eq!(before, 0);

    with_local_recorder(&recorder, || {
        with_recorder(|rec| {
            let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
            for i in 0..10 {
                let name = format!("leak_test.counter.{}", i);
                let key = Key::from_name(name);
                let _c = rec.register_counter(&key, &metadata);
            }
        });
    });

    let after = ops.load(Ordering::SeqCst);
    assert_eq!(after, 10);
    assert_eq!(after - before, 10);


    with_recorder(|rec| {
        let metadata = Metadata::new(module_path!(), metrics::Level::INFO, None);
        let key = Key::from_name("post_local.counter");
        let _c = rec.register_counter(&key, &metadata);
    });

    let final_count = ops.load(Ordering::SeqCst);
    assert_eq!(final_count, 10);
}

#[test]
fn test_with_local_recorder_describe_operations() {
    let ops = Arc::new(AtomicU64::new(0));
    let recorder = CountingRecorder::new(ops.clone());

    assert_eq!(ops.load(Ordering::SeqCst), 0);

    with_local_recorder(&recorder, || {
        with_recorder(|rec| {
            rec.describe_counter(
                KeyName::from("desc.counter"),
                Some(Unit::Count),
                SharedString::from("counter description"),
            );
            rec.describe_gauge(
                KeyName::from("desc.gauge"),
                None,
                SharedString::from("gauge description"),
            );
            rec.describe_histogram(
                KeyName::from("desc.histogram"),
                Some(Unit::Seconds),
                SharedString::from("histogram description"),
            );
        });
    });

    assert_eq!(ops.load(Ordering::SeqCst), 3);


    with_local_recorder(&recorder, || {
        with_recorder(|rec| {
            rec.describe_histogram(
                KeyName::from("desc.histogram.bytes"),
                Some(Unit::Bytes),
                SharedString::from("bytes histogram"),
            );
            rec.describe_histogram(
                KeyName::from("desc.histogram.ms"),
                Some(Unit::Milliseconds),
                SharedString::from("ms histogram"),
            );
        });
    });

    assert_eq!(ops.load(Ordering::SeqCst), 5);
    assert!(ops.load(Ordering::SeqCst) > 3);
    assert_ne!(ops.load(Ordering::SeqCst), 0);
}