use curl::easy::{Easy2, Handler, WriteError};
use curl::multi::Multi;

struct Sink(Vec<u8>);

impl Handler for Sink {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
}

#[test]
fn test_multi_setopt_workflow() {
    curl::init();
    let mut multi = Multi::new();


    assert!(multi.pipelining(false, false).is_ok());
    assert!(multi.pipelining(true, false).is_ok());
    assert!(multi.pipelining(false, true).is_ok());
    assert!(multi.pipelining(true, true).is_ok());


    assert!(multi.set_max_host_connections(0).is_ok());
    assert!(multi.set_max_host_connections(1).is_ok());
    assert!(multi.set_max_host_connections(8).is_ok());
    assert!(multi.set_max_host_connections(64).is_ok());

    assert!(multi.set_max_total_connections(0).is_ok());
    assert!(multi.set_max_total_connections(1).is_ok());
    assert!(multi.set_max_total_connections(16).is_ok());
    assert!(multi.set_max_total_connections(128).is_ok());

    assert!(multi.set_max_connects(1).is_ok());
    assert!(multi.set_max_connects(10).is_ok());
    assert!(multi.set_max_connects(100).is_ok());

    assert!(multi.set_pipeline_length(1).is_ok());
    assert!(multi.set_pipeline_length(5).is_ok());
    assert!(multi.set_pipeline_length(20).is_ok());
}

#[test]
fn test_multi_callback_registration() {
    curl::init();
    let mut multi = Multi::new();


    assert!(multi.socket_function(|_socket, _events, _token| ()).is_ok());


    assert!(multi.timer_function(|_timeout| true).is_ok());


    assert!(multi.socket_function(|_, _, _| ()).is_ok());
    assert!(multi.timer_function(|_| false).is_ok());


    let t1 = multi.get_timeout().expect("get_timeout");
    let t2 = multi.get_timeout().expect("get_timeout");
    assert_eq!(t1, t2);

    assert!(t1.is_none() || t1.unwrap().as_secs() < 60);


    let max_fd = multi.fdset2(None, None, None).expect("fdset2");
    assert!(max_fd.is_none() || max_fd.unwrap() >= -1);


    let mut called = 0u32;
    multi.messages(|_msg| {
        called += 1;
    });
    assert_eq!(called, 0);
}

#[test]
fn test_multi_add2_remove2_roundtrip() {
    curl::init();
    let multi = Multi::new();


    let mut easy = Easy2::new(Sink(Vec::new()));
    let pre_len = easy.get_ref().0.len();
    assert_eq!(pre_len, 0);
    let _ = easy.url("http://127.0.0.1:1/");


    let handle = multi.add2(easy).expect("add2");


    assert_eq!(handle.get_ref().0.len(), 0);
    assert!(handle.get_ref().0.is_empty());


    let recovered = multi.remove2(handle).expect("remove2");
    assert_eq!(recovered.get_ref().0.len(), 0);
    assert!(recovered.get_ref().0.is_empty());


    let easy2 = Easy2::new(Sink(Vec::new()));
    let handle2 = multi.add2(easy2).expect("add2 second");
    assert_eq!(handle2.get_ref().0.len(), 0);
    let recovered2 = multi.remove2(handle2).expect("remove2 second");
    assert_eq!(recovered2.get_ref().0.len(), 0);


    let mut count = 0u32;
    multi.messages(|_| {
        count += 1;
    });
    assert_eq!(count, 0);
}

#[test]
fn test_multi_full_configuration_workflow() {
    curl::init();
    let mut multi = Multi::new();


    assert!(multi.pipelining(true, true).is_ok());
    assert!(multi.set_max_host_connections(4).is_ok());
    assert!(multi.set_max_total_connections(16).is_ok());
    assert!(multi.set_max_connects(32).is_ok());
    assert!(multi.set_pipeline_length(4).is_ok());


    assert!(multi.socket_function(|_, _, _| ()).is_ok());
    assert!(multi.timer_function(|_| true).is_ok());


    let easy = Easy2::new(Sink(Vec::new()));
    let handle = multi.add2(easy).expect("add2");
    assert_eq!(handle.get_ref().0.len(), 0);

    let t = multi.get_timeout().expect("get_timeout after add");

    if let Some(d) = t {
        assert!(d.as_secs() < 3600);
    }

    let max_fd = multi.fdset2(None, None, None).expect("fdset2 after add");
    assert!(max_fd.is_none() || max_fd.unwrap() >= -1);


    let mut n_msgs = 0usize;
    multi.messages(|_| {
        n_msgs += 1;
    });
    assert_eq!(n_msgs, 0);


    let recovered = multi.remove2(handle).expect("remove2");
    assert_eq!(recovered.get_ref().0.len(), 0);
    assert!(recovered.get_ref().0.is_empty());
}