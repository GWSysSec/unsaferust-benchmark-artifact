#[test]
fn format_bytes_builds_immutable_bytes_from_mixed_arguments() {
    let user = "alice";
    let count = 3;
    let payload = bytes_utils::format_bytes!("user={}; count={}; status={}", user, count, "ok");

    assert_eq!(&payload[..], "user=alice; count=3; status=ok");
    assert_eq!(payload.len(), 30);
    assert!(!payload.is_empty());

    let text = &payload[..];
    assert_eq!(text, "user=alice; count=3; status=ok");

    let cloned = payload.clone();
    assert_eq!(&cloned[..], &payload[..]);
}

#[test]
fn format_bytes_handles_empty_and_unicode_content() {
    let empty = bytes_utils::format_bytes!("");
    assert_eq!(empty.len(), 0);
    assert_eq!(&empty[..], "");

    let unicode = bytes_utils::format_bytes!("{}:{}:{}", "snowman", '☃', 42);
    let unicode_text = &unicode[..];

    assert_eq!(unicode_text, "snowman:☃:42");
    assert_eq!(unicode_text.chars().count(), 12);
    assert!(unicode.len() > unicode_text.chars().count());
}

#[test]
fn format_bytes_mut_builds_mutable_buffer_that_can_be_extended() {
    use std::fmt::Write as _;

    let mut buf = bytes_utils::format_bytes_mut!("prefix-{}", 10);

    assert_eq!(&buf[..], "prefix-10");
    assert_eq!(buf.len(), 9);

    write!(&mut buf, "-middle").expect("writing to mutable formatted buffer should succeed");
    assert_eq!(&buf[..], "prefix-10-middle");

    let suffix = bytes_utils::format_bytes!("-{}", "suffix");
    write!(&mut buf, "{}", &suffix[..]).expect("writing suffix should succeed");
    assert_eq!(&buf[..], "prefix-10-middle-suffix");

    let text = &buf[..];
    assert_eq!(text, "prefix-10-middle-suffix");
}

#[test]
fn format_bytes_and_format_bytes_mut_interoperate_in_workflow() {
    use std::fmt::Write as _;

    let header = bytes_utils::format_bytes!("id={}", 7);
    let body = bytes_utils::format_bytes!("name={}", "example");

    let mut message = bytes_utils::format_bytes_mut!("{};", &header[..]);
    write!(&mut message, "{}", &body[..]).expect("writing body should succeed");

    assert_eq!(&message[..], "id=7;name=example");

    let message_text = &message[..];
    let parts: Vec<&str> = message_text.split(';').collect();

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "id=7");
    assert_eq!(parts[1], "name=example");
    assert!(message_text.contains("example"));
}