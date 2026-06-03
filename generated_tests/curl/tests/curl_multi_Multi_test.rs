use std::fs;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use curl::easy::{Easy2, Handler, WriteError};
use curl::multi::Multi;

struct CollectingHandler {
    body: Arc<Mutex<Vec<u8>>>,
}

impl Handler for CollectingHandler {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.body.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    }
}

fn temp_file_url(contents: &[u8]) -> (std::path::PathBuf, String) {
    let name = format!(
        "curl_multi_easy2_integration_{}_{}.txt",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    );
    let path = std::env::temp_dir().join(name);
    fs::write(&path, contents).unwrap();
    let url = format!("file://{}", path.to_str().unwrap());
    (path, url)
}

#[test]
fn multi_easy2_callbacks_configuration_and_completion_workflow() {
    curl::init();

    let payload = b"multi easy2 transfer through public API\nwith deterministic file data\n";
    let (path, url) = temp_file_url(payload);

    let socket_callback_count = Arc::new(AtomicUsize::new(0));
    let timer_callback_count = Arc::new(AtomicUsize::new(0));
    let timer_values = Arc::new(Mutex::new(Vec::new()));

    let mut multi = Multi::new();

    {
        let socket_callback_count = Arc::clone(&socket_callback_count);
        curl::multi::Multi::socket_function(&mut multi, move |_socket, events, token| {
            assert_eq!(token, 0);
            let interested_in_something = events.input() || events.output() || events.remove();
            assert!(interested_in_something);
            socket_callback_count.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
    }

    {
        let timer_callback_count = Arc::clone(&timer_callback_count);
        let timer_values = Arc::clone(&timer_values);
        curl::multi::Multi::timer_function(&mut multi, move |timeout| {
            timer_callback_count.fetch_add(1, Ordering::SeqCst);
            timer_values.lock().unwrap().push(timeout);
            true
        })
        .unwrap();
    }

    curl::multi::Multi::pipelining(&mut multi, false, true).unwrap();
    curl::multi::Multi::set_max_host_connections(&mut multi, 2).unwrap();
    curl::multi::Multi::set_max_total_connections(&mut multi, 4).unwrap();
    curl::multi::Multi::set_max_connects(&mut multi, 3).unwrap();
    curl::multi::Multi::set_pipeline_length(&mut multi, 2).unwrap();

    let initial_timeout = curl::multi::Multi::get_timeout(&multi).unwrap();
    assert!(
        initial_timeout.is_none() || initial_timeout.unwrap() <= Duration::from_secs(10),
        "unexpectedly large initial timeout: {:?}",
        initial_timeout
    );

    let fdset_result = curl::multi::Multi::fdset2(&multi, None, None, None).unwrap();
    assert_eq!(fdset_result, None);

    let body = Arc::new(Mutex::new(Vec::new()));
    let handler = CollectingHandler {
        body: Arc::clone(&body),
    };
    let mut easy = Easy2::new(handler);
    easy.url(&url).unwrap();
    easy.follow_location(false).unwrap();

    let handle = curl::multi::Multi::add2(&multi, easy).unwrap();

    let after_add_timeout = curl::multi::Multi::get_timeout(&multi).unwrap();
    assert!(
        after_add_timeout.is_none() || after_add_timeout.unwrap() <= Duration::from_secs(10),
        "unexpectedly large timeout after add2: {:?}",
        after_add_timeout
    );

    let mut observed_running = Vec::new();
    loop {
        let running = multi.perform().unwrap();
        observed_running.push(running);
        if running == 0 {
            break;
        }
        multi.wait(&mut [], Duration::from_millis(50)).unwrap();
    }

    assert!(
        !observed_running.is_empty(),
        "perform loop should run at least once"
    );
    assert_eq!(
        *observed_running.last().unwrap(),
        0,
        "final perform call should report no running transfers"
    );

    let mut message_count = 0usize;
    curl::multi::Multi::messages(&multi, |_| {
        message_count += 1;
    });
    assert!(
        message_count >= 1,
        "completed transfer should leave at least one multi message"
    );

    let easy = curl::multi::Multi::remove2(&multi, handle).unwrap();
    drop(easy);

    assert_eq!(&*body.lock().unwrap(), payload);
    assert!(
        timer_callback_count.load(Ordering::SeqCst) >= 1,
        "adding/running a transfer should notify the timer callback"
    );
    assert!(
        !timer_values.lock().unwrap().is_empty(),
        "timer callback should record at least one timeout update"
    );

    let final_fdset_result = curl::multi::Multi::fdset2(&multi, None, None, None).unwrap();
    assert_eq!(final_fdset_result, None);

    fs::remove_file(path).unwrap();
}