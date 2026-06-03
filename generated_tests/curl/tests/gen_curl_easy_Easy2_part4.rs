use curl::easy::{Easy2, Handler, TimeCondition, WriteError};
use std::time::Duration;

struct Sink(Vec<u8>);

impl Handler for Sink {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

#[test]
fn test_http_decoding_and_request_method_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.http_content_decoding(true).is_ok());
    assert!(easy.http_content_decoding(false).is_ok());
    assert!(easy.http_content_decoding(true).is_ok());


    assert!(easy.http_transfer_decoding(true).is_ok());
    assert!(easy.http_transfer_decoding(false).is_ok());
    assert!(easy.http_transfer_decoding(true).is_ok());


    assert!(easy.custom_request("GET").is_ok());
    assert!(easy.custom_request("POST").is_ok());
    assert!(easy.custom_request("PUT").is_ok());
    assert!(easy.custom_request("DELETE").is_ok());
    assert!(easy.custom_request("PATCH").is_ok());
    assert!(easy.custom_request("PROPFIND").is_ok());


    assert!(easy.nobody(true).is_ok());
    assert!(easy.nobody(false).is_ok());


    assert!(easy.fetch_filetime(true).is_ok());
    assert!(easy.fetch_filetime(false).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_range_resume_and_size_limits_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());


    assert!(easy.range("0-99").is_ok());
    assert!(easy.range("100-199").is_ok());
    assert!(easy.range("0-0,1024-2047").is_ok());
    assert!(easy.range("500-").is_ok());


    assert!(easy.resume_from(0).is_ok());
    assert!(easy.resume_from(1).is_ok());
    assert!(easy.resume_from(1024).is_ok());
    assert!(easy.resume_from(1024 * 1024).is_ok());
    assert!(easy.resume_from(u64::from(u32::MAX)).is_ok());


    assert!(easy.max_filesize(0).is_ok());
    assert!(easy.max_filesize(1024).is_ok());
    assert!(easy.max_filesize(1024 * 1024).is_ok());
    assert!(easy.max_filesize(u64::from(u32::MAX)).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
}

#[test]
fn test_time_condition_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);


    let tc = TimeCondition::IfModifiedSince;
    let tc2 = tc.clone();
    assert!(easy.time_condition(tc).is_ok());
    assert!(easy.time_condition(tc2).is_ok());
    assert!(easy.time_condition(TimeCondition::IfModifiedSince).is_ok());
    assert!(easy.time_condition(TimeCondition::IfUnmodifiedSince).is_ok());
    assert!(easy.time_condition(TimeCondition::None).is_ok());


    assert!(easy.time_value(0).is_ok());
    assert!(easy.time_value(1).is_ok());
    assert!(easy.time_value(1_000_000_000).is_ok());
    assert!(easy.time_value(1_700_000_000).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}

#[test]
fn test_speed_and_connection_limits_workflow() {
    curl::init();
    let mut easy = Easy2::new(Sink(Vec::new()));

    assert_eq!(easy.get_ref().0.len(), 0);


    assert!(easy.low_speed_limit(0).is_ok());
    assert!(easy.low_speed_limit(1).is_ok());
    assert!(easy.low_speed_limit(1024).is_ok());
    assert!(easy.low_speed_limit(1024 * 1024).is_ok());

    assert!(easy.low_speed_time(Duration::from_secs(1)).is_ok());
    assert!(easy.low_speed_time(Duration::from_secs(30)).is_ok());
    assert!(easy.low_speed_time(Duration::from_secs(300)).is_ok());


    assert!(easy.max_send_speed(0).is_ok());
    assert!(easy.max_send_speed(1024).is_ok());
    assert!(easy.max_send_speed(1024 * 1024).is_ok());
    assert!(easy.max_send_speed(10 * 1024 * 1024).is_ok());

    assert!(easy.max_recv_speed(0).is_ok());
    assert!(easy.max_recv_speed(1024).is_ok());
    assert!(easy.max_recv_speed(1024 * 1024).is_ok());
    assert!(easy.max_recv_speed(10 * 1024 * 1024).is_ok());


    assert!(easy.max_connects(0).is_ok());
    assert!(easy.max_connects(1).is_ok());
    assert!(easy.max_connects(10).is_ok());
    assert!(easy.max_connects(100).is_ok());


    assert!(easy.custom_request("GET").is_ok());
    assert!(easy.nobody(true).is_ok());
    assert!(easy.fetch_filetime(true).is_ok());
    assert!(easy.max_filesize(1_000_000).is_ok());
    assert!(easy.range("0-1023").is_ok());
    assert!(easy.resume_from(0).is_ok());
    assert!(easy.http_content_decoding(true).is_ok());
    assert!(easy.http_transfer_decoding(true).is_ok());
    assert!(easy.time_condition(TimeCondition::None).is_ok());
    assert!(easy.time_value(0).is_ok());


    assert_eq!(easy.get_ref().0.len(), 0);
    assert!(easy.get_ref().0.is_empty());
}