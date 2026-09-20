
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

struct MockBody {
    size_hint: SizeHint,
}

impl Body for MockBody {
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

    let hint2 = SizeHint::new();
    assert_eq!(hint2.lower(), 0);
    assert_eq!(hint2.upper(), None);

    // Default and new should produce equivalent results
    assert_eq!(hint.lower(), hint2.lower());
    assert_eq!(hint.upper(), hint2.upper());

    // Clone should preserve values
    let hint3 = hint.clone();
    assert_eq!(hint3.lower(), 0);
    assert_eq!(hint3.upper(), None);
}

#[test]
fn size_hint_set_lower() {
    let mut hint = SizeHint::new();
    assert_eq!(hint.lower(), 0);

    hint.set_lower(42);
    assert_eq!(hint.lower(), 42);
    assert_eq!(hint.upper(), None);

    hint.set_lower(0);
    assert_eq!(hint.lower(), 0);

    hint.set_lower(u64::MAX);
    assert_eq!(hint.lower(), u64::MAX);

    hint.set_lower(1024);
    assert_eq!(hint.lower(), 1024);

    hint.set_lower(100);
    assert_eq!(hint.lower(), 100);

    // Verify upper is still unset
    assert_eq!(hint.upper(), None);

    hint.set_lower(0);
    assert_eq!(hint.lower(), 0);
}

#[test]
fn size_hint_set_upper() {
    let mut hint = SizeHint::new();
    assert_eq!(hint.upper(), None);

    hint.set_upper(100);
    assert_eq!(hint.upper(), Some(100));
    assert_eq!(hint.lower(), 0);

    hint.set_upper(0);
    assert_eq!(hint.upper(), Some(0));

    hint.set_upper(u64::MAX);
    assert_eq!(hint.upper(), Some(u64::MAX));

    hint.set_upper(512);
    assert_eq!(hint.upper(), Some(512));

    // Lower should remain unchanged
    assert_eq!(hint.lower(), 0);

    hint.set_upper(1);
    assert_eq!(hint.upper(), Some(1));

    hint.set_upper(999_999_999);
    assert_eq!(hint.upper(), Some(999_999_999));
}

#[test]
fn size_hint_set_exact() {
    let mut hint = SizeHint::new();
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), None);

    hint.set_exact(256);
    assert_eq!(hint.lower(), 256);
    assert_eq!(hint.upper(), Some(256));

    hint.set_exact(0);
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), Some(0));

    hint.set_exact(u64::MAX);
    assert_eq!(hint.lower(), u64::MAX);
    assert_eq!(hint.upper(), Some(u64::MAX));

    // After set_exact, lower == upper
    hint.set_exact(1024);
    assert_eq!(hint.lower(), 1024);
    assert_eq!(hint.upper(), Some(1024));
    assert_eq!(hint.lower(), hint.upper().unwrap());
}

#[test]
fn size_hint_with_bounds_combination() {
    let mut hint = SizeHint::new();

    // Set lower first, then upper
    hint.set_lower(10);
    hint.set_upper(100);
    assert_eq!(hint.lower(), 10);
    assert_eq!(hint.upper(), Some(100));

    // Overwrite with exact
    hint.set_exact(50);
    assert_eq!(hint.lower(), 50);
    assert_eq!(hint.upper(), Some(50));

    // Then change lower independently
    hint.set_lower(20);
    assert_eq!(hint.lower(), 20);
    assert_eq!(hint.upper(), Some(50));

    // Then change upper independently
    hint.set_upper(200);
    assert_eq!(hint.lower(), 20);
    assert_eq!(hint.upper(), Some(200));
}

#[test]
fn size_hint_clone_independence() {
    let mut hint1 = SizeHint::new();
    hint1.set_lower(100);
    hint1.set_upper(500);

    let mut hint2 = hint1.clone();
    assert_eq!(hint2.lower(), 100);
    assert_eq!(hint2.upper(), Some(500));

    // Mutating clone should not affect original
    hint2.set_lower(200);
    hint2.set_upper(600);
    assert_eq!(hint1.lower(), 100);
    assert_eq!(hint1.upper(), Some(500));
    assert_eq!(hint2.lower(), 200);
    assert_eq!(hint2.upper(), Some(600));

    // Mutating original should not affect clone
    hint1.set_exact(300);
    assert_eq!(hint1.lower(), 300);
    assert_eq!(hint1.upper(), Some(300));
    assert_eq!(hint2.lower(), 200);
    assert_eq!(hint2.upper(), Some(600));
}

#[test]
fn size_hint_with_body_trait() {
    let mut hint = SizeHint::new();
    hint.set_lower(64);
    hint.set_upper(128);

    let mock = MockBody {
        size_hint: hint.clone(),
    };

    let body_hint = Body::size_hint(&mock);
    assert_eq!(body_hint.lower(), 64);
    assert_eq!(body_hint.upper(), Some(128));

    // Verify is_end_stream behavior with non-zero bounds
    let mut mock2 = MockBody {
        size_hint: hint.clone(),
    };
    assert!(!Pin::new(&mut mock2).is_end_stream());

    // Zero exact should still not be end_stream by default poll behavior
    let mut zero_hint = SizeHint::new();
    zero_hint.set_exact(0);
    let mut mock3 = MockBody {
        size_hint: zero_hint,
    };
    assert!(!Pin::new(&mut mock3).is_end_stream());

    // Verify the hint values are preserved through the body
    let body_hint2 = Body::size_hint(&mock);
    assert_eq!(body_hint2.lower(), body_hint.lower());
    assert_eq!(body_hint2.upper(), body_hint.upper());
}

#[test]
fn size_hint_boundary_values() {
    let mut hint = SizeHint::new();

    // Test with 0
    hint.set_exact(0);
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), Some(0));

    // Test with 1
    hint.set_exact(1);
    assert_eq!(hint.lower(), 1);
    assert_eq!(hint.upper(), Some(1));

    // Test with large power of 2
    hint.set_exact(1 << 40);
    assert_eq!(hint.lower(), 1 << 40);
    assert_eq!(hint.upper(), Some(1 << 40));

    // Test with u64::MAX
    hint.set_exact(u64::MAX);
    assert_eq!(hint.lower(), u64::MAX);
    assert_eq!(hint.upper(), Some(u64::MAX));

    // Test with u64::MAX - 1
    hint.set_lower(u64::MAX - 1);
    assert_eq!(hint.lower(), u64::MAX - 1);

    hint.set_upper(u64::MAX - 1);
    assert_eq!(hint.upper(), Some(u64::MAX - 1));

    // Verify we can go back to zero
    hint.set_exact(0);
    assert_eq!(hint.lower(), 0);
    assert_eq!(hint.upper(), Some(0));
}

#[test]
fn size_hint_multiple_mutations_sequence() {
    let mut hint = SizeHint::new();

    // Sequence of mutations
    hint.set_lower(10);
    assert_eq!(hint.lower(), 10);

    hint.set_upper(20);
    assert_eq!(hint.upper(), Some(20));

    hint.set_exact(15);
    assert_eq!(hint.lower(), 15);
    assert_eq!(hint.upper(), Some(15));

    hint.set_lower(5);
    assert_eq!(hint.lower(), 5);
    assert_eq!(hint.upper(), Some(15));

    hint.set_upper(50);
    assert_eq!(hint.upper(), Some(50));
    assert_eq!(hint.lower(), 5);

    hint.set_exact(25);
    assert_eq!(hint.lower(), 25);
    assert_eq!(hint.upper(), Some(25));

    // Rapid lower changes
    for i in 0..10u64 {
        hint.set_lower(i);
        assert_eq!(hint.lower(), i);
    }
}

#[test]
fn size_hint_debug_format() {
    let mut hint = SizeHint::new();
    hint.set_lower(10);
    hint.set_upper(100);

    let debug_str = format!("{:?}", hint);
    // Debug output should contain the values
    assert!(debug_str.contains("10"), "debug should contain lower bound");
    assert!(debug_str.contains("100"), "debug should contain upper bound");

    let default_hint = SizeHint::default();
    let default_debug = format!("{:?}", default_hint);
    assert!(default_debug.contains("0"), "default debug should contain 0");

    // Exact hint debug
    let mut exact_hint = SizeHint::new();
    exact_hint.set_exact(42);
    let exact_debug = format!("{:?}", exact_hint);
    assert!(exact_debug.contains("42"), "exact debug should contain 42");

    // Verify debug doesn't panic on boundary values
    let mut max_hint = SizeHint::new();
    max_hint.set_exact(u64::MAX);
    let max_debug = format!("{:?}", max_hint);
    assert!(!max_debug.is_empty());
}