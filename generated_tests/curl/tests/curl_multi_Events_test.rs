use curl::easy::Easy;
use curl::multi::Events;

#[test]
fn events_error_builder_chains_on_same_events_value() {
    curl::init();

    let mut events = Events::new();
    let original = &mut events as *mut Events;

    let returned = Events::error(&mut events, true);
    assert_eq!(returned as *mut Events, original);

    let returned = Events::error(returned, false);
    assert_eq!(returned as *mut Events, original);

    let returned = Events::error(returned, true);
    assert_eq!(returned as *mut Events, original);

    let returned = Events::error(returned, true);
    assert_eq!(returned as *mut Events, original);

    let returned = Events::error(returned, false);
    assert_eq!(returned as *mut Events, original);
}

#[test]
fn events_error_can_be_configured_alongside_real_easy_setup() -> Result<(), Box<dyn std::error::Error>> {
    curl::init();

    let mut events = Events::new();
    let events_addr = &mut events as *mut Events;

    let returned = Events::error(&mut events, true);
    assert_eq!(returned as *mut Events, events_addr);

    let returned = Events::error(returned, false);
    assert_eq!(returned as *mut Events, events_addr);

    let mut easy = Easy::new();
    assert!(!easy.raw().is_null());

    let url = "https://example.invalid/events-error-integration";
    easy.url(url)?;
    easy.verbose(false)?;
    easy.show_header(false)?;
    easy.progress(false)?;
    easy.fail_on_error(true)?;

    assert_eq!(easy.effective_url()?, Some(url));
    assert_eq!(easy.effective_url_bytes()?, Some(url.as_bytes()));

    let encoded = easy.url_encode(b"events error: true/false & spaces");
    assert!(encoded.contains("events"));
    assert!(encoded.contains("%20"));

    let decoded = easy.url_decode(&encoded);
    assert_eq!(decoded, b"events error: true/false & spaces");

    Ok(())
}