#[cfg(all(unix, feature = "parallel"))]
mod unix_parallel_stderr_tests {
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn nonblocking_stderr_reports_available_bytes_and_allows_incremental_read() {
        let expected = b"cc-rs stderr availability probe\n";

        let mut child = Command::new("sh")
            .arg("-c")
            .arg("printf 'cc-rs stderr availability probe\n' >&2; sleep 0.2")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn shell test process");

        let mut stderr = child.stderr.take().expect("child stderr was not piped");

        cc::parallel::stderr::set_non_blocking(&stderr)
            .expect("setting child stderr to non-blocking should succeed");

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut available = 0usize;
        while Instant::now() < deadline {
            available = cc::parallel::stderr::bytes_available(&mut stderr)
                .expect("querying available stderr bytes should succeed");
            if available > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(available > 0, "stderr should eventually contain bytes");
        assert!(
            available <= expected.len(),
            "reported bytes should not exceed the known message length"
        );

        let mut buf = vec![0u8; available];
        let read = stderr
            .read(&mut buf)
            .expect("non-blocking read after bytes_available should succeed");

        assert!(read > 0, "at least one byte should be readable");
        assert!(
            read <= available,
            "read count should not exceed bytes_available result"
        );
        assert!(
            expected.starts_with(&buf[..read]),
            "read data should be a prefix of the emitted stderr message"
        );

        let status = child.wait().expect("waiting for child should succeed");
        assert!(status.success(), "shell test process should exit successfully");
    }

    #[test]
    fn nonblocking_stderr_reports_zero_for_quiet_live_process() {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("sleep 0.2")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn quiet shell test process");

        let mut stderr = child.stderr.take().expect("child stderr was not piped");

        cc::parallel::stderr::set_non_blocking(&stderr)
            .expect("setting quiet child stderr to non-blocking should succeed");

        let available = cc::parallel::stderr::bytes_available(&mut stderr)
            .expect("querying quiet stderr should succeed");

        assert_eq!(
            available, 0,
            "quiet process should not have stderr bytes available immediately"
        );

        let mut one_byte = [0u8; 1];
        let read_result = stderr.read(&mut one_byte);
        assert!(
            matches!(
                read_result,
                Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock
            ) || matches!(read_result, Ok(0)),
            "non-blocking read from quiet live stderr should not produce data"
        );

        let status = child.wait().expect("waiting for quiet child should succeed");
        assert!(status.success(), "quiet shell test process should exit successfully");
    }
}

#[cfg(not(all(unix, feature = "parallel")))]
#[test]
fn parallel_stderr_helpers_require_unix_and_parallel_feature() {
    assert!(true);
}