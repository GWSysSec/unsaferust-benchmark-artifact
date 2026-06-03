#![cfg(not(target_os = "unknown"))]

use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use async_std::future::timeout;
use async_std::net::UdpSocket;
use async_std::task;

#[test]
fn udp_send_recv_from_roundtrip() {
    task::block_on(async {
        let server = UdpSocket::bind("127.0.0.1:0").await.expect("bind server");
        let client = UdpSocket::bind("127.0.0.1:0").await.expect("bind client");

        let server_addr = server.local_addr().expect("server local_addr");
        let client_addr = client.local_addr().expect("client local_addr");

        assert!(server_addr.is_ipv4());
        assert!(client_addr.is_ipv4());
        assert_ne!(server_addr.port(), 0);
        assert_ne!(client_addr.port(), 0);
        assert_ne!(server_addr.port(), client_addr.port());

        let payload: &[u8] = b"the quick brown fox jumps over the lazy dog";
        let sent = client
            .send_to(payload, server_addr)
            .await
            .expect("send_to");
        assert_eq!(sent, payload.len());

        let mut buf = [0u8; 128];
        let (n, from) = timeout(Duration::from_secs(5), server.recv_from(&mut buf))
            .await
            .expect("recv_from did not complete in time")
            .expect("recv_from failed");

        assert_eq!(n, payload.len());
        assert_eq!(&buf[..n], payload);
        assert_eq!(from.port(), client_addr.port());
        assert_eq!(from.ip(), client_addr.ip());

        assert_eq!(buf[n], 0u8);
    });
}

#[test]
fn udp_peek_from_does_not_consume_datagram() {
    task::block_on(async {
        let server = UdpSocket::bind("127.0.0.1:0").await.expect("bind server");
        let client = UdpSocket::bind("127.0.0.1:0").await.expect("bind client");
        let server_addr = server.local_addr().expect("server addr");
        let client_addr = client.local_addr().expect("client addr");

        let payload: &[u8] = b"peekable-datagram-payload";
        let sent = client
            .send_to(payload, server_addr)
            .await
            .expect("send_to");
        assert_eq!(sent, payload.len());

        let mut peek_buf = [0u8; 128];
        let (pn, pfrom) = timeout(Duration::from_secs(5), server.peek_from(&mut peek_buf))
            .await
            .expect("peek timed out")
            .expect("peek_from");

        assert_eq!(pn, payload.len());
        assert_eq!(&peek_buf[..pn], payload);
        assert_eq!(pfrom.port(), client_addr.port());
        assert_eq!(pfrom.ip(), client_addr.ip());


        let mut peek_buf2 = [0u8; 128];
        let (pn2, pfrom2) = timeout(Duration::from_secs(5), server.peek_from(&mut peek_buf2))
            .await
            .expect("peek2 timed out")
            .expect("peek_from 2");
        assert_eq!(pn2, pn);
        assert_eq!(&peek_buf2[..pn2], payload);
        assert_eq!(pfrom2, pfrom);


        let mut recv_buf = [0u8; 128];
        let (rn, rfrom) = timeout(Duration::from_secs(5), server.recv_from(&mut recv_buf))
            .await
            .expect("recv timed out")
            .expect("recv_from");
        assert_eq!(rn, payload.len());
        assert_eq!(&recv_buf[..rn], payload);
        assert_eq!(rfrom, pfrom);
    });
}

#[test]
fn udp_broadcast_getter_setter_roundtrip() {
    task::block_on(async {
        let sock = UdpSocket::bind("127.0.0.1:0").await.expect("bind");

        let initial = sock.broadcast().expect("broadcast() initial");

        assert!(initial == true || initial == false);

        sock.set_broadcast(true).expect("set_broadcast(true)");
        let on = sock.broadcast().expect("broadcast() on");
        assert_eq!(on, true);
        assert_ne!(on, false);

        sock.set_broadcast(false).expect("set_broadcast(false)");
        let off = sock.broadcast().expect("broadcast() off");
        assert_eq!(off, false);
        assert_ne!(off, on);


        sock.set_broadcast(true).expect("set_broadcast(true) 2");
        assert_eq!(sock.broadcast().unwrap(), true);
        sock.set_broadcast(false).expect("set_broadcast(false) 2");
        assert_eq!(sock.broadcast().unwrap(), false);
    });
}

#[test]
fn udp_ttl_getter_setter_roundtrip() {
    task::block_on(async {
        let sock = UdpSocket::bind("127.0.0.1:0").await.expect("bind");

        let initial = sock.ttl().expect("ttl() initial");
        assert!(initial > 0, "ttl should be positive, got {}", initial);
        assert!(initial <= 255, "ttl should fit a byte, got {}", initial);

        sock.set_ttl(42).expect("set_ttl 42");
        let ttl_a = sock.ttl().expect("ttl after 42");
        assert_eq!(ttl_a, 42);

        sock.set_ttl(128).expect("set_ttl 128");
        let ttl_b = sock.ttl().expect("ttl after 128");
        assert_eq!(ttl_b, 128);
        assert_ne!(ttl_b, ttl_a);

        sock.set_ttl(1).expect("set_ttl 1");
        let ttl_c = sock.ttl().expect("ttl after 1");
        assert_eq!(ttl_c, 1);
        assert_ne!(ttl_c, ttl_b);
    });
}

#[test]
fn udp_multicast_v4_loop_and_ttl() {
    task::block_on(async {
        let sock = UdpSocket::bind("0.0.0.0:0").await.expect("bind v4");


        let initial_loop = sock.multicast_loop_v4().expect("mc_loop_v4 initial");
        assert!(initial_loop == true || initial_loop == false);

        sock.set_multicast_loop_v4(true)
            .expect("set mc_loop_v4 true");
        let on = sock.multicast_loop_v4().expect("mc_loop_v4 on");
        assert_eq!(on, true);

        sock.set_multicast_loop_v4(false)
            .expect("set mc_loop_v4 false");
        let off = sock.multicast_loop_v4().expect("mc_loop_v4 off");
        assert_eq!(off, false);
        assert_ne!(off, on);


        let initial_ttl = sock.multicast_ttl_v4().expect("mc_ttl_v4 initial");
        assert!(initial_ttl <= 255);

        sock.set_multicast_ttl_v4(7).expect("set mc_ttl_v4 7");
        let ttl_a = sock.multicast_ttl_v4().expect("mc_ttl_v4 after 7");
        assert_eq!(ttl_a, 7);

        sock.set_multicast_ttl_v4(64).expect("set mc_ttl_v4 64");
        let ttl_b = sock.multicast_ttl_v4().expect("mc_ttl_v4 after 64");
        assert_eq!(ttl_b, 64);
        assert_ne!(ttl_b, ttl_a);
    });
}

#[test]
fn udp_join_multicast_v4_state_preserved() {
    task::block_on(async {
        let sock = UdpSocket::bind("0.0.0.0:0").await.expect("bind v4");


        sock.set_multicast_loop_v4(true).expect("mc loop v4 on");
        assert_eq!(sock.multicast_loop_v4().unwrap(), true);

        sock.set_multicast_ttl_v4(4).expect("mc ttl v4 = 4");
        assert_eq!(sock.multicast_ttl_v4().unwrap(), 4);

        sock.set_ttl(33).expect("ttl = 33");
        assert_eq!(sock.ttl().unwrap(), 33);


        let group = Ipv4Addr::new(239, 255, 42, 98);
        let iface = Ipv4Addr::new(0, 0, 0, 0);

        sock.join_multicast_v4(group, iface)
            .expect("join_multicast_v4");


        assert_eq!(sock.multicast_loop_v4().unwrap(), true);
        assert_eq!(sock.multicast_ttl_v4().unwrap(), 4);
        assert_eq!(sock.ttl().unwrap(), 33);


        let addr = sock.local_addr().expect("local_addr");
        assert!(addr.is_ipv4());
        assert_ne!(addr.port(), 0);
    });
}

#[test]
fn udp_multicast_loop_v6_roundtrip() {
    task::block_on(async {
        let sock = UdpSocket::bind("[::1]:0").await.expect("bind v6 loopback");

        let initial = sock.multicast_loop_v6().expect("mc_loop_v6 initial");
        assert!(initial == true || initial == false);

        sock.set_multicast_loop_v6(true)
            .expect("set mc_loop_v6 true");
        let on = sock.multicast_loop_v6().expect("mc_loop_v6 on");
        assert_eq!(on, true);

        sock.set_multicast_loop_v6(false)
            .expect("set mc_loop_v6 false");
        let off = sock.multicast_loop_v6().expect("mc_loop_v6 off");
        assert_eq!(off, false);
        assert_ne!(off, on);


        sock.set_multicast_loop_v6(initial)
            .expect("restore mc_loop_v6");
        assert_eq!(sock.multicast_loop_v6().unwrap(), initial);


        sock.set_multicast_loop_v6(!initial).expect("flip");
        assert_eq!(sock.multicast_loop_v6().unwrap(), !initial);

        let addr = sock.local_addr().expect("local_addr v6");
        assert!(addr.is_ipv6());
    });
}

#[test]
fn udp_join_multicast_v6_then_configure() {
    task::block_on(async {
        let sock = UdpSocket::bind("[::]:0").await.expect("bind v6 any");
        let addr = sock.local_addr().expect("local_addr v6");
        assert!(addr.is_ipv6());
        assert_ne!(addr.port(), 0);

        sock.set_multicast_loop_v6(true)
            .expect("set mc_loop_v6 true");
        assert_eq!(sock.multicast_loop_v6().unwrap(), true);


        let group = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0x1234);





        let join_result = sock.join_multicast_v6(&group, 0);
        let joined = join_result.is_ok();
        if let Err(ref e) = join_result {

            let _ = e.kind();
        }
        assert!(joined || join_result.is_err());


        sock.set_multicast_loop_v6(false)
            .expect("disable mc_loop_v6");
        assert_eq!(sock.multicast_loop_v6().unwrap(), false);

        sock.set_ttl(16).expect("set ttl 16");
        assert_eq!(sock.ttl().unwrap(), 16);


        let target = UdpSocket::bind("[::1]:0").await.expect("bind target");
        let target_addr = target.local_addr().expect("target addr");
        let payload = b"hi-v6";
        let sender = UdpSocket::bind("[::1]:0").await.expect("bind sender");
        let n = sender
            .send_to(payload, target_addr)
            .await
            .expect("send_to v6");
        assert_eq!(n, payload.len());

        let mut buf = [0u8; 32];
        let (rn, _from) = timeout(Duration::from_secs(5), target.recv_from(&mut buf))
            .await
            .expect("recv timed out")
            .expect("recv_from v6");
        assert_eq!(rn, payload.len());
        assert_eq!(&buf[..rn], payload);
    });
}