use curl::easy::{Easy, Form};

fn collect_form_error_flags(error: &curl::FormError) -> Vec<(&'static str, bool)> {
    vec![
        ("is_memory", curl::FormError::is_memory(error)),
        (
            "is_option_twice",
            curl::FormError::is_option_twice(error),
        ),
        ("is_null", curl::FormError::is_null(error)),
        (
            "is_unknown_option",
            curl::FormError::is_unknown_option(error),
        ),
        ("is_incomplete", curl::FormError::is_incomplete(error)),
        (
            "is_illegal_array",
            curl::FormError::is_illegal_array(error),
        ),
        ("is_disabled", curl::FormError::is_disabled(error)),
    ]
}

#[test]
fn incomplete_form_part_reports_expected_form_error_metadata() {
    curl::init();

    let mut form = Form::new();
    let result = form.part("field_without_payload").add();

    let error = match result {
        Ok(()) => panic!("adding a form part with only a field name should be rejected"),
        Err(error) => error,
    };

    assert!(curl::FormError::is_incomplete(&error));
    assert!(!curl::FormError::is_memory(&error));
    assert!(!curl::FormError::is_option_twice(&error));
    assert!(!curl::FormError::is_null(&error));
    assert!(!curl::FormError::is_unknown_option(&error));
    assert!(!curl::FormError::is_illegal_array(&error));
    assert!(!curl::FormError::is_disabled(&error));

    let flags = collect_form_error_flags(&error);
    let active_flags: Vec<&'static str> = flags
        .iter()
        .filter_map(|(name, active)| if *active { Some(*name) } else { None })
        .collect();

    assert_eq!(active_flags, vec!["is_incomplete"]);

    let description = curl::FormError::description(&error);
    assert!(!description.trim().is_empty());
    assert!(
        description.to_ascii_lowercase().contains("incomplete")
            || description.to_ascii_lowercase().contains("form")
    );

    let first_code = curl::FormError::code(&error);
    let second_code = curl::FormError::code(&error);
    assert_eq!(first_code, second_code);
}

#[test]
fn form_error_does_not_poison_later_valid_form_and_easy_configuration(
) -> Result<(), Box<dyn std::error::Error>> {
    curl::init();

    let mut invalid_form = Form::new();
    let error = invalid_form
        .part("missing_contents")
        .add()
        .expect_err("incomplete form part should produce a FormError");

    assert!(curl::FormError::is_incomplete(&error));
    assert_eq!(curl::FormError::code(&error), curl::FormError::code(&error));
    assert!(!curl::FormError::description(&error).is_empty());

    let mut valid_form = Form::new();
    valid_form
        .part("message")
        .contents(b"hello from integration test")
        .add()?;
    valid_form.part("empty_but_explicit").contents(b"").add()?;

    let mut easy = Easy::new();
    let url = "https://example.invalid/form-error-recovery";
    easy.url(url)?;
    easy.post(true)?;
    easy.httppost(valid_form)?;
    easy.useragent("curl-rust-form-error-integration-test/1.0")?;

    assert_eq!(easy.effective_url()?, Some(url));
    assert_eq!(easy.effective_url_bytes()?, Some(url.as_bytes()));
    assert_eq!(easy.response_code()?, 0);
    assert_eq!(easy.upload_size()?, 0.0);
    assert_eq!(easy.download_size()?, 0.0);

    Ok(())
}