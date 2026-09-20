use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

struct SizeHintMock {
    size_hint: SizeHint,
}

impl Body for SizeHintMock {
    type Data = ::std::io::Cursor<Vec<u8>>;
    type Error = ();

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(None)
    }

    fn size_hint(&self) -> SizeHint {
        self.size_hint.clone()
    }
}

#[test]
fn size_hint_default_values() {
    let hint = SizeHint::default();
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), None);

    let mock = SizeHintMock { size_hint: hint };
    let returned_hint = mock.size_hint();
    assert_eq!(returned_hint.lower(), 0);
    assert_eq!(returned_hint.upper(), None);

    let hint2 = SizeHint::new();
    assert_eq!(hint2.lower(), 0);
    assert_eq!(hint2.upper(), None);

    let mock2 = SizeHintMock { size_hint: hint2 };
    let returned_hint2 = mock2.size_hint();
    assert_eq!(returned_hint2.lower(), 0);
    assert_eq!(returned_hint2.upper(), None);
}

#[test]
fn size_hint_with_lower_only() {
    let mut hint = SizeHint::new();
    hint.set_lower(42);
    assert_eq!(hint.lower(), 42);
    assert_eq!(hint.upper(), None);

    let mock = SizeHintMock { size_hint: hint };
    let returned = mock.size_hint();
    assert_eq!(returned.lower(), 42);
    assert_eq!(returned.upper(), None);

    let mut hint2 = SizeHint::new();
    hint2.set_lower(0);
    assert_eq!(hint2.lower(), 0);

    let mut hint3 = SizeHint::new();
    hint3.set_lower(u64::MAX);
    assert_eq!(hint3.lower(), u64::MAX);
}

#[test]
fn size_hint_with_upper_only() {
    let mut hint = SizeHint::new();
    hint.set_upper(100);
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), Some(100));

    let mock = SizeHintMock { size_hint: hint };
    let returned = mock.size_hint();
    assert_eq!(returned.lower(), 0);
    assert_eq!(returned.upper(), Some(100));

    let mut hint2 = SizeHint::new();
    hint2.set_upper(0);
    assert_eq!(hint2.lower(), 0);
    assert_eq!(hint2.upper(), Some(0));

    let mut hint3 = SizeHint::new();
    hint3.set_upper(u64::MAX);
    assert_eq!(hint3.upper(), Some(u64::MAX));
}

#[test]
fn size_hint_with_exact() {
    let mut hint = SizeHint::new();
    hint.set_exact(256);
    assert_eq!(hint.lower(), 256);
    assert_eq!(hint.upper(), Some(256));

    let mock = SizeHintMock { size_hint: hint };
    let returned = mock.size_hint();
    assert_eq!(returned.lower(), 256);
    assert_eq!(returned.upper(), Some(256));

    let mut hint2 = SizeHint::new();
    hint2.set_exact(0);
    assert_eq!(hint2.lower(), 0);
    assert_eq!(hint2.upper(), Some(0));

    let mut hint3 = SizeHint::new();
    hint3.set_exact(u64::MAX);
    assert_eq!(hint3.lower(), u64::MAX);
    assert_eq!(hint3.upper(), Some(u64::MAX));
}

#[test]
fn size_hint_with_lower_and_upper() {
    let mut hint = SizeHint::new();
    hint.set_lower(10);
    hint.set_upper(500);
    assert_eq!(hint.lower(), 10);
    assert_eq!(hint.upper(), Some(500));

    let mock = SizeHintMock { size_hint: hint };
    let returned = mock.size_hint();
    assert_eq!(returned.lower(), 10);
    assert_eq!(returned.upper(), Some(500));

    let mut hint2 = SizeHint::new();
    hint2.set_lower(100);
    hint2.set_upper(100);
    assert_eq!(hint2.lower(), 100);
    assert_eq!(hint2.upper(), Some(100));

    let mut hint3 = SizeHint::new();
    hint3.set_lower(0);
    hint3.set_upper(u64::MAX);
    assert_eq!(hint3.lower(), 0);
    assert_eq!(hint3.upper(), Some(u64::MAX));
}

#[test]
fn size_hint_mutation_sequence() {
    let mut hint = SizeHint::new();
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), None);

    hint.set_lower(50);
    assert_eq!(hint.lower(), 50);
    assert_eq!(hint.upper(), None);

    hint.set_upper(200);
    assert_eq!(hint.lower(), 50);
    assert_eq!(hint.upper(), Some(200));

    hint.set_exact(75);
    assert_eq!(hint.lower(), 75);
    assert_eq!(hint.upper(), Some(75));

    let mock = SizeHintMock { size_hint: hint.clone() };
    let returned = mock.size_hint();
    assert_eq!(returned.lower(), 75);
    assert_eq!(returned.upper(), Some(75));
}

#[test]
fn size_hint_clone_independence() {
    let mut hint = SizeHint::new();
    hint.set_lower(10);
    hint.set_upper(20);

    let cloned = hint.clone();
    assert_eq!(cloned.lower(), 10);
    assert_eq!(cloned.upper(), Some(20));

    // set_lower has an assertion that value <= self.upper.unwrap_or(u64::MAX)
    // We must set upper first (or remove it) before setting lower to a value
    // that exceeds the current upper.
    hint.set_upper(999);
    hint.set_lower(99);
    assert_eq!(hint.lower(), 99);
    assert_eq!(hint.upper(), Some(999));
    assert_eq!(cloned.lower(), 10);
    assert_eq!(cloned.upper(), Some(20));

    let mock = SizeHintMock { size_hint: cloned };
    let returned = mock.size_hint();
    assert_eq!(returned.lower(), 10);
    assert_eq!(returned.upper(), Some(20));
}

#[test]
fn size_hint_body_trait_method_called_via_trait() {
    let mut hint = SizeHint::new();
    hint.set_lower(1024);
    hint.set_upper(4096);

    let mock = SizeHintMock { size_hint: hint };

    let body_ref: &dyn Body<Data = std::io::Cursor<Vec<u8>>, Error = ()> = &mock;
    let returned = body_ref.size_hint();
    assert_eq!(returned.lower(), 1024);
    assert_eq!(returned.upper(), Some(4096));

    let mut hint2 = SizeHint::new();
    hint2.set_exact(512);
    let mock2 = SizeHintMock { size_hint: hint2 };
    let body_ref2: &dyn Body<Data = std::io::Cursor<Vec<u8>>, Error = ()> = &mock2;
    let returned2 = body_ref2.size_hint();
    assert_eq!(returned2.lower(), 512);
    assert_eq!(returned2.upper(), Some(512));

    let mock3 = SizeHintMock { size_hint: SizeHint::new() };
    let body_ref3: &dyn Body<Data = std::io::Cursor<Vec<u8>>, Error = ()> = &mock3;
    let returned3 = body_ref3.size_hint();
    assert_eq!(returned3.lower(), 0);
    assert_eq!(returned3.upper(), None);
}

#[test]
fn size_hint_boundary_values() {
    let mut hint = SizeHint::new();
    hint.set_exact(0);
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), Some(0));

    let mock = SizeHintMock { size_hint: hint };
    let returned = mock.size_hint();
    assert_eq!(returned.lower(), 0);
    assert_eq!(returned.upper(), Some(0));

    let mut hint_max = SizeHint::new();
    hint_max.set_exact(u64::MAX);
    assert_eq!(hint_max.lower(), u64::MAX);
    assert_eq!(hint_max.upper(), Some(u64::MAX));

    let mock_max = SizeHintMock { size_hint: hint_max };
    let returned_max = mock_max.size_hint();
    assert_eq!(returned_max.lower(), u64::MAX);
    assert_eq!(returned_max.upper(), Some(u64::MAX));
}

#[test]
fn size_hint_multiple_bodies_independent() {
    let mut hint_a = SizeHint::new();
    hint_a.set_lower(100);
    hint_a.set_upper(200);

    let mut hint_b = SizeHint::new();
    hint_b.set_exact(50);

    let mock_a = SizeHintMock { size_hint: hint_a };
    let mock_b = SizeHintMock { size_hint: hint_b };

    let returned_a = mock_a.size_hint();
    let returned_b = mock_b.size_hint();

    assert_eq!(returned_a.lower(), 100);
    assert_eq!(returned_a.upper(), Some(200));
    assert_eq!(returned_b.lower(), 50);
    assert_eq!(returned_b.upper(), Some(50));

    assert_ne!(returned_a.lower(), returned_b.lower());
    assert_ne!(returned_a.upper(), returned_b.upper());
}