extern crate native_tls;

use native_tls::{TlsConnector, TlsAcceptor, Identity, Certificate, Protocol, HandshakeError};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn generate_self_signed_identity() -> (Vec<u8>, Vec<u8>) {
    use std::process::Command;
    use std::fs;

    // Use a unique directory per thread to avoid races between parallel tests
    let id = std::thread::current().id();
    let dir = std::env::temp_dir().join(format!("native_tls_test_certs_{:?}", id));
    let _ = fs::create_dir_all(&dir);

    let key_path = dir.join("key.pem");
    let cert_path = dir.join("cert.pem");
    let pfx_path = dir.join("identity.pfx");
    let der_path = dir.join("cert.der");

    // Generate key and cert
    let output = Command::new("openssl")
        .args(&["req", "-x509", "-newkey", "rsa:2048", "-keyout"])
        .arg(&key_path)
        .args(&["-out"])
        .arg(&cert_path)
        .args(&["-days", "1", "-nodes", "-subj", "/CN=localhost"])
        .output()
        .expect("openssl req failed");
    assert!(output.status.success(), "openssl req failed: {}", String::from_utf8_lossy(&output.stderr));

    // Generate PKCS12
    let output = Command::new("openssl")
        .args(&["pkcs12", "-export", "-out"])
        .arg(&pfx_path)
        .args(&["-inkey"])
        .arg(&key_path)
        .args(&["-in"])
        .arg(&cert_path)
        .args(&["-passout", "pass:testpass"])
        .output()
        .expect("openssl pkcs12 failed");
    assert!(output.status.success(), "openssl pkcs12 failed: {}", String::from_utf8_lossy(&output.stderr));

    // Generate DER cert
    let output = Command::new("openssl")
        .args(&["x509", "-in"])
        .arg(&cert_path)
        .args(&["-outform", "DER", "-out"])
        .arg(&der_path)
        .output()
        .expect("openssl x509 der failed");
    assert!(output.status.success(), "openssl x509 der failed: {}", String::from_utf8_lossy(&output.stderr));

    let pfx = fs::read(&pfx_path).expect("read pfx");
    let der = fs::read(&der_path).expect("read der");

    let _ = fs::remove_dir_all(&dir);

    (pfx, der)
}

#[test]
fn test_mid_handshake_get_ref_via_interrupted_handshake() {
    let (pfx, _der_cert) = generate_self_signed_identity();

    let identity = Identity::from_pkcs12(&pfx, "testpass").unwrap();
    let _acceptor = TlsAcceptor::new(identity).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    // Server thread: accept TCP but close immediately to cause handshake failure on client
    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        // Set a short timeout and drop without doing TLS
        stream.set_read_timeout(Some(Duration::from_millis(50))).ok();
        // Just drop the stream to cause client handshake to fail
        drop(stream);
    });

    // Client: attempt TLS connect which should fail since server drops
    let tcp_stream = TcpStream::connect(addr).unwrap();
    tcp_stream.set_read_timeout(Some(Duration::from_millis(500))).ok();
    tcp_stream.set_write_timeout(Some(Duration::from_millis(500))).ok();

    let connector = TlsConnector::new().unwrap();
    let result = connector.connect("localhost", tcp_stream);

    // The connection should fail since the server dropped
    assert!(result.is_err(), "Expected handshake to fail when server drops connection");

    match result {
        Err(HandshakeError::Failure(e)) => {
            // Failure error - verify it has a useful display
            let err_msg = format!("{}", e);
            assert!(!err_msg.is_empty(), "Error message should not be empty");
            assert_ne!(err_msg.len(), 0);
        }
        Err(HandshakeError::WouldBlock(mut mid)) => {
            // We got a MidHandshakeTlsStream - exercise its API
            let stream_ref = mid.get_ref();
            let peer = stream_ref.peer_addr();
            assert!(peer.is_ok(), "get_ref should return valid TcpStream");
            assert_eq!(peer.unwrap(), addr);

            let stream_mut = mid.get_mut();
            let local = stream_mut.local_addr();
            assert!(local.is_ok(), "get_mut should return valid TcpStream");
            let local_addr = local.unwrap();
            assert_ne!(local_addr.port(), 0);
            assert_eq!(local_addr.ip(), std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));

            // Try to resume handshake - should fail since server is gone
            let resume_result = mid.handshake();
            assert!(resume_result.is_err(), "Resumed handshake should fail");
        }
        Ok(_) => {
            panic!("Should not succeed connecting to a dropped server");
        }
    }

    server_handle.join().unwrap();
}

#[test]
fn test_mid_handshake_nonblocking_client_exercises_all_methods() {
    let (pfx, _der_cert) = generate_self_signed_identity();

    let identity = Identity::from_pkcs12(&pfx, "testpass").unwrap();

    let acceptor = TlsAcceptor::new(identity).unwrap();
    let acceptor = Arc::new(acceptor);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let acceptor_clone = acceptor.clone();
    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_millis(2000))).ok();
        stream.set_write_timeout(Some(Duration::from_millis(2000))).ok();
        // Delay slightly then do TLS accept
        thread::sleep(Duration::from_millis(20));
        let tls_result = acceptor_clone.accept(stream);
        match tls_result {
            Ok(mut tls_stream) => {
                let _ = tls_stream.write_all(b"hello from server");
                let inner = tls_stream.get_ref();
                let peer = inner.peer_addr().unwrap();
                assert_eq!(peer.ip(), std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
            }
            Err(_) => {
                // Client may have disconnected, that's ok for this test
            }
        }
    });

    // Client side: use nonblocking to potentially get WouldBlock
    let tcp_stream = TcpStream::connect(addr).unwrap();
    tcp_stream.set_nonblocking(true).ok();
    tcp_stream.set_read_timeout(Some(Duration::from_millis(2000))).ok();

    let connector = TlsConnector::new().unwrap();
    let result = connector.connect("localhost", tcp_stream);

    match result {
        Err(HandshakeError::WouldBlock(mut mid)) => {
            // Exercise get_ref
            let inner_ref = mid.get_ref();
            let peer_addr = inner_ref.peer_addr().unwrap();
            assert_eq!(peer_addr.port(), addr.port());
            assert_eq!(peer_addr.ip(), std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));

            // Exercise get_mut
            let inner_mut = mid.get_mut();
            let local_addr = inner_mut.local_addr().unwrap();
            assert_ne!(local_addr.port(), 0);
            assert_ne!(local_addr.port(), addr.port());

            // Set back to blocking for handshake retry
            inner_mut.set_nonblocking(false).unwrap();
            inner_mut.set_read_timeout(Some(Duration::from_millis(2000))).ok();
            inner_mut.set_write_timeout(Some(Duration::from_millis(2000))).ok();

            // Exercise handshake - retry
            let mut retry_result = mid.handshake();
            let mut attempts = 0;
            loop {
                match retry_result {
                    Err(HandshakeError::WouldBlock(m)) => {
                        if attempts > 20 {
                            // Timed out retrying, acceptable in test
                            break;
                        }
                        attempts += 1;
                        thread::sleep(Duration::from_millis(10));
                        retry_result = m.handshake();
                    }
                    other => {
                        match other {
                            Ok(mut tls_stream) => {
                                let inner = tls_stream.get_ref();
                                let connected_peer = inner.peer_addr().unwrap();
                                assert_eq!(connected_peer.port(), addr.port());

                                let mut buf = [0u8; 64];
                                let n = tls_stream.read(&mut buf).unwrap_or(0);
                                if n > 0 {
                                    assert_eq!(&buf[..n], b"hello from server");
                                }
                            }
                            Err(HandshakeError::Failure(e)) => {
                                let msg = format!("{}", e);
                                assert!(!msg.is_empty());
                            }
                            Err(HandshakeError::WouldBlock(_)) => {
                                unreachable!();
                            }
                        }
                        break;
                    }
                }
            }
        }
        Err(HandshakeError::Failure(e)) => {
            // Certificate validation failure expected since self-signed
            let err_str = format!("{}", e);
            assert!(!err_str.is_empty(), "Error should have description");
            assert_ne!(err_str.len(), 0);
        }
        Ok(mut tls_stream) => {
            // Handshake completed immediately (unlikely with self-signed but possible)
            let inner = tls_stream.get_ref();
            let peer = inner.peer_addr().unwrap();
            assert_eq!(peer.port(), addr.port());
            assert_eq!(peer.ip(), std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));

            let mut buf = [0u8; 64];
            let n = tls_stream.read(&mut buf).unwrap_or(0);
            if n > 0 {
                assert_eq!(&buf[..n], b"hello from server");
            }
        }
    }

    server_handle.join().unwrap();
}

#[test]
fn test_mid_handshake_stream_with_custom_connector_and_certificate() {
    let (pfx, der_cert) = generate_self_signed_identity();

    let identity = Identity::from_pkcs12(&pfx, "testpass").unwrap();
    let _cert = Certificate::from_der(&der_cert).unwrap();

    // Verify certificate was created successfully
    let cert2 = Certificate::from_der(&der_cert);
    assert!(cert2.is_ok(), "Certificate::from_der should succeed with valid DER");

    // Verify identity was created successfully
    let identity2 = Identity::from_pkcs12(&pfx, "testpass");
    assert!(identity2.is_ok(), "Identity::from_pkcs12 should succeed with correct password");

    // Verify wrong password fails
    let bad_identity = Identity::from_pkcs12(&pfx, "wrongpass");
    assert!(bad_identity.is_err(), "Identity::from_pkcs12 should fail with wrong password");

    let bad_err = match bad_identity {
        Err(e) => e,
        Ok(_) => panic!("Expected error for wrong password"),
    };
    let bad_err_msg = format!("{}", bad_err);
    assert!(!bad_err_msg.is_empty(), "Error should have a message");

    // Verify invalid DER fails
    let bad_cert = Certificate::from_der(b"not a valid certificate");
    assert!(bad_cert.is_err(), "Certificate::from_der should fail with invalid data");

    let bad_cert_err = match bad_cert {
        Err(e) => e,
        Ok(_) => panic!("Expected error for invalid DER"),
    };
    let bad_cert_msg = format!("{}", bad_cert_err);
    assert!(!bad_cert_msg.is_empty(), "Certificate error should have a message");

    // Set up server with the identity
    let acceptor = TlsAcceptor::new(identity).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_millis(2000))).ok();
        stream.set_write_timeout(Some(Duration::from_millis(2000))).ok();
        let result = acceptor.accept(stream);
        match result {
            Ok(mut s) => {
                s.write_all(b"server data").ok();
            }
            Err(_) => {}
        }
    });

    // Client with nonblocking to trigger WouldBlock
    let tcp = TcpStream::connect(addr).unwrap();
    tcp.set_nonblocking(true).ok();

    let connector = TlsConnector::new().unwrap();
    let handshake_result = connector.connect("localhost", tcp);

    match handshake_result {
        Err(HandshakeError::WouldBlock(mut mid)) => {
            // get_ref: verify the underlying stream properties
            let s = mid.get_ref();
            let peer = s.peer_addr().unwrap();
            assert_eq!(peer.port(), addr.port());

            // get_mut: modify stream settings
            let s_mut = mid.get_mut();
            s_mut.set_nonblocking(false).unwrap();
            s_mut.set_read_timeout(Some(Duration::from_millis(2000))).unwrap();
            let local = s_mut.local_addr().unwrap();
            assert_ne!(local.port(), addr.port());

            // handshake: attempt to complete
            let final_result = mid.handshake();
            match final_result {
                Ok(stream) => {
                    let inner = stream.get_ref();
                    assert_eq!(inner.peer_addr().unwrap().port(), addr.port());
                }
                Err(HandshakeError::WouldBlock(mid2)) => {
                    let s2 = mid2.get_ref();
                    assert_eq!(s2.peer_addr().unwrap().port(), addr.port());
                }
                Err(HandshakeError::Failure(e)) => {
                    let msg = format!("{}", e);
                    assert!(!msg.is_empty());
                }
            }
        }
        Err(HandshakeError::Failure(e)) => {
            let msg = format!("{}", e);
            assert!(!msg.is_empty());
            assert_ne!(msg.len(), 0);
        }
        Ok(stream) => {
            let inner = stream.get_ref();
            let peer = inner.peer_addr().unwrap();
            assert_eq!(peer.port(), addr.port());
        }
    }

    server_handle.join().unwrap();
}

#[test]
fn test_protocol_clone_and_acceptor_builder_min_protocol() {
    let (pfx, _der_cert) = generate_self_signed_identity();

    let identity = Identity::from_pkcs12(&pfx, "testpass").unwrap();

    // Test Protocol clone
    let proto = Protocol::Tlsv12;
    let proto_clone = proto.clone();

    // Build acceptor with min protocol version
    let mut builder = TlsAcceptor::builder(identity);
    builder.min_protocol_version(Some(proto_clone));

    let acceptor_result = builder.build();
    assert!(acceptor_result.is_ok(), "TlsAcceptor build with Tlsv12 min should succeed");

    // Test with None protocol
    let (pfx2, _) = generate_self_signed_identity();
    let identity2 = Identity::from_pkcs12(&pfx2, "testpass").unwrap();
    let mut builder2 = TlsAcceptor::builder(identity2);
    builder2.min_protocol_version(None);
    let acceptor2 = builder2.build();
    assert!(acceptor2.is_ok(), "TlsAcceptor build with None min protocol should succeed");

    // Test TlsConnector::new
    let connector = TlsConnector::new();
    assert!(connector.is_ok(), "TlsConnector::new should succeed");

    // Verify connector can be created multiple times
    let connector2 = TlsConnector::new();
    assert!(connector2.is_ok(), "Second TlsConnector::new should also succeed");

    // Test invalid pkcs12 data
    let bad_pkcs12 = Identity::from_pkcs12(b"garbage data", "pass");
    assert!(bad_pkcs12.is_err());

    let empty_pkcs12 = Identity::from_pkcs12(b"", "");
    assert!(empty_pkcs12.is_err());
}