#![cfg(unix)]

use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use curl::easy::{Easy, Easy2, Handler, WriteError};
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

fn file_url(path: &std::path::Path) -> String {
    format!("file://{}", path.to_str().expect("temporary path is valid UTF-8"))
}

#[test]
fn multi_messages_match_easy_and_easy2_handles_with_tokens() {
    curl::init();

    let unique = format!(
        "curl_multi_message_result_for_{}_{}.txt",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    );
    let path = std::env::temp_dir().join(unique);
    let payload = b"multi message APIs should identify their originating handles\n";
    fs::write(&path, payload).unwrap();

    let easy_body = Arc::new(Mutex::new(Vec::new()));
    let easy2_body = Arc::new(Mutex::new(Vec::new()));

    let mut easy = Easy::new();
    easy.url(&file_url(&path)).unwrap();
    easy.fail_on_error(true).unwrap();
    easy.follow_location(false).unwrap();

    let easy_body_for_callback = Arc::clone(&easy_body);
    easy.write_function(move |data| {
        easy_body_for_callback.lock().unwrap().extend_from_slice(data);
        Ok(data.len())
    })
    .unwrap();

    let mut easy2 = Easy2::new(CollectingHandler {
        body: Arc::clone(&easy2_body),
    });
    easy2.url(&file_url(&path)).unwrap();
    easy2.fail_on_error(true).unwrap();
    easy2.follow_location(false).unwrap();

    let multi = Multi::new();

    let mut easy_handle = multi.add(easy).unwrap();
    easy_handle.set_token(0xEAF0).unwrap();

    let mut easy2_handle = multi.add2(easy2).unwrap();
    easy2_handle.set_token(0xE2F0).unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_easy_message = false;
    let mut saw_easy2_message = false;

    while !(saw_easy_message && saw_easy2_message) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for both multi completion messages"
        );

        while multi.perform().unwrap() > 0 {
            multi.wait(&mut [], Duration::from_millis(100)).unwrap();
        }

        multi.messages(|message| {
            if curl::multi::Message::is_for(&message, &easy_handle) {
                assert!(
                    !saw_easy_message,
                    "received more than one completion message for Easy handle"
                );
                assert!(curl::multi::Message::is_for(&message, &easy_handle));
                assert!(!curl::multi::Message::is_for2(&message, &easy2_handle));
                assert_eq!(curl::multi::Message::token(&message).unwrap(), 0xEAF0);

                let easy_result = curl::multi::Message::result_for(&message, &easy_handle);
                assert!(easy_result.is_some());
                assert!(easy_result.unwrap().is_ok());

                let easy2_result = curl::multi::Message::result_for2(&message, &easy2_handle);
                assert!(easy2_result.is_none());

                saw_easy_message = true;
            } else if curl::multi::Message::is_for2(&message, &easy2_handle) {
                assert!(
                    !saw_easy2_message,
                    "received more than one completion message for Easy2 handle"
                );
                assert!(curl::multi::Message::is_for2(&message, &easy2_handle));
                assert!(!curl::multi::Message::is_for(&message, &easy_handle));
                assert_eq!(curl::multi::Message::token(&message).unwrap(), 0xE2F0);

                let easy2_result = curl::multi::Message::result_for2(&message, &easy2_handle);
                assert!(easy2_result.is_some());
                assert!(easy2_result.unwrap().is_ok());

                let easy_result = curl::multi::Message::result_for(&message, &easy_handle);
                assert!(easy_result.is_none());

                saw_easy2_message = true;
            } else {
                panic!("completion message did not match either registered handle");
            }
        });

        if !(saw_easy_message && saw_easy2_message) {
            multi.wait(&mut [], Duration::from_millis(50)).unwrap();
        }
    }

    assert_eq!(&*easy_body.lock().unwrap(), payload);
    assert_eq!(&*easy2_body.lock().unwrap(), payload);

    drop(easy_handle);
    drop(easy2_handle);
    let _ = fs::remove_file(path);
}