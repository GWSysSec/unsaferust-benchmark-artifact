
use miette::{Diagnostic, LabeledSpan, MietteDiagnostic, Severity, SourceSpan};

#[test]
fn test_miette_diagnostic_with_code_builder() {
    let diag = MietteDiagnostic::new("something went wrong")
        .with_code("E001");

    assert_eq!(diag.message, "something went wrong");
    assert_eq!(diag.code.as_deref(), Some("E001"));
    assert_eq!(diag.severity, None);
    assert_eq!(diag.help.as_deref(), None);
    assert_eq!(diag.url.as_deref(), None);
    assert!(diag.labels.is_none() || diag.labels.as_ref().unwrap().is_empty());

    let diag2 = MietteDiagnostic::new("another error")
        .with_code("module::submodule::E042".to_string());
    assert_eq!(diag2.code.as_deref(), Some("module::submodule::E042"));
    assert_eq!(diag2.message, "another error");
}

#[test]
fn test_miette_diagnostic_with_severity_variants() {
    let error_diag = MietteDiagnostic::new("critical failure")
        .with_severity(Severity::Error);
    assert_eq!(error_diag.severity, Some(Severity::Error));
    assert_eq!(error_diag.message, "critical failure");

    let warning_diag = MietteDiagnostic::new("potential issue")
        .with_severity(Severity::Warning);
    assert_eq!(warning_diag.severity, Some(Severity::Warning));
    assert_eq!(warning_diag.message, "potential issue");

    let advice_diag = MietteDiagnostic::new("consider this")
        .with_severity(Severity::Advice);
    assert_eq!(advice_diag.severity, Some(Severity::Advice));
    assert_eq!(advice_diag.message, "consider this");

    assert_ne!(error_diag.severity, warning_diag.severity);
    assert_ne!(warning_diag.severity, advice_diag.severity);
    assert_ne!(error_diag.severity, advice_diag.severity);
}

#[test]
fn test_miette_diagnostic_with_help() {
    let diag = MietteDiagnostic::new("file not found")
        .with_help("check that the file path is correct");

    assert_eq!(diag.message, "file not found");
    assert_eq!(diag.help.as_deref(), Some("check that the file path is correct"));
    assert_eq!(diag.code, None);
    assert_eq!(diag.severity, None);
    assert_eq!(diag.url, None);

    let diag2 = MietteDiagnostic::new("parse error")
        .with_help("ensure the input is valid JSON".to_string());
    assert_eq!(diag2.help.as_deref(), Some("ensure the input is valid JSON"));
    assert_eq!(diag2.message, "parse error");
}

#[test]
fn test_miette_diagnostic_with_url() {
    let diag = MietteDiagnostic::new("deprecated API usage")
        .with_url("https://docs.example.com/migration-guide");

    assert_eq!(diag.message, "deprecated API usage");
    assert_eq!(diag.url.as_deref(), Some("https://docs.example.com/migration-guide"));
    assert_eq!(diag.code, None);
    assert_eq!(diag.help, None);
    assert_eq!(diag.severity, None);

    let diag2 = MietteDiagnostic::new("unknown option")
        .with_url("https://example.com/docs/options".to_string());
    assert_eq!(diag2.url.as_deref(), Some("https://example.com/docs/options"));
    assert_eq!(diag2.message, "unknown option");
}

#[test]
fn test_miette_diagnostic_with_label_single() {
    let label = LabeledSpan::new(Some("here".to_string()), 5, 10);
    let diag = MietteDiagnostic::new("syntax error")
        .with_label(label);

    assert_eq!(diag.message, "syntax error");
    let labels = diag.labels.as_ref().unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].label(), Some("here"));
    let inner_span: &SourceSpan = labels[0].inner();
    assert_eq!(inner_span.offset(), 5);
    assert_eq!(inner_span.len(), 10);
    assert_eq!(diag.code, None);
    assert_eq!(diag.severity, None);
    assert_eq!(diag.help, None);
}

#[test]
fn test_miette_diagnostic_with_labels_multiple() {
    let label1 = LabeledSpan::new(Some("first".to_string()), 0, 5);
    let label2 = LabeledSpan::new(Some("second".to_string()), 10, 3);
    let label3 = LabeledSpan::new(None, 20, 7);

    let diag = MietteDiagnostic::new("multiple issues found")
        .with_labels(vec![label1, label2, label3]);

    assert_eq!(diag.message, "multiple issues found");
    let labels = diag.labels.as_ref().unwrap();
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0].label(), Some("first"));
    assert_eq!(labels[1].label(), Some("second"));
    assert_eq!(labels[2].label(), None);
    let span0: &SourceSpan = labels[0].inner();
    assert_eq!(span0.offset(), 0);
    assert_eq!(span0.len(), 5);
    let span1: &SourceSpan = labels[1].inner();
    assert_eq!(span1.offset(), 10);
    assert_eq!(span1.len(), 3);
    let span2: &SourceSpan = labels[2].inner();
    assert_eq!(span2.offset(), 20);
    assert_eq!(span2.len(), 7);
}

#[test]
fn test_miette_diagnostic_and_label_appends() {
    let label1 = LabeledSpan::new(Some("start".to_string()), 0, 4);
    let label2 = LabeledSpan::new(Some("end".to_string()), 50, 6);

    let diag = MietteDiagnostic::new("range error")
        .with_label(label1)
        .and_label(label2);

    assert_eq!(diag.message, "range error");
    let labels = diag.labels.as_ref().unwrap();
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].label(), Some("start"));
    assert_eq!(labels[1].label(), Some("end"));
    let span0: &SourceSpan = labels[0].inner();
    assert_eq!(span0.offset(), 0);
    assert_eq!(span0.len(), 4);
    let span1: &SourceSpan = labels[1].inner();
    assert_eq!(span1.offset(), 50);
    assert_eq!(span1.len(), 6);
}

#[test]
fn test_miette_diagnostic_and_labels_appends_batch() {
    let initial_label = LabeledSpan::new(Some("initial".to_string()), 0, 2);
    let extra1 = LabeledSpan::new(Some("extra1".to_string()), 10, 3);
    let extra2 = LabeledSpan::new(Some("extra2".to_string()), 20, 4);
    let extra3 = LabeledSpan::new(Some("extra3".to_string()), 30, 5);

    let diag = MietteDiagnostic::new("accumulated errors")
        .with_label(initial_label)
        .and_labels(vec![extra1, extra2, extra3]);

    assert_eq!(diag.message, "accumulated errors");
    let labels = diag.labels.as_ref().unwrap();
    assert_eq!(labels.len(), 4);
    assert_eq!(labels[0].label(), Some("initial"));
    assert_eq!(labels[1].label(), Some("extra1"));
    assert_eq!(labels[2].label(), Some("extra2"));
    assert_eq!(labels[3].label(), Some("extra3"));
    let span3: &SourceSpan = labels[3].inner();
    assert_eq!(span3.offset(), 30);
    assert_eq!(span3.len(), 5);
}

#[test]
fn test_miette_diagnostic_full_builder_chain() {
    let diag = MietteDiagnostic::new("type mismatch")
        .with_code("E0308")
        .with_severity(Severity::Error)
        .with_help("expected `u32`, found `&str`")
        .with_url("https://doc.rust-lang.org/error-index.html#E0308")
        .with_label(LabeledSpan::new(Some("expected u32".to_string()), 12, 8))
        .and_label(LabeledSpan::new(Some("found &str".to_string()), 25, 5));

    assert_eq!(diag.message, "type mismatch");
    assert_eq!(diag.code.as_deref(), Some("E0308"));
    assert_eq!(diag.severity, Some(Severity::Error));
    assert_eq!(diag.help.as_deref(), Some("expected `u32`, found `&str`"));
    assert_eq!(diag.url.as_deref(), Some("https://doc.rust-lang.org/error-index.html#E0308"));

    let labels = diag.labels.as_ref().unwrap();
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].label(), Some("expected u32"));
    assert_eq!(labels[1].label(), Some("found &str"));
    let span0: &SourceSpan = labels[0].inner();
    assert_eq!(span0.offset(), 12);
    assert_eq!(span0.len(), 8);
    let span1: &SourceSpan = labels[1].inner();
    assert_eq!(span1.offset(), 25);
    assert_eq!(span1.len(), 5);
}

#[test]
fn test_miette_diagnostic_with_labels_replaces_previous() {
    let label_a = LabeledSpan::new(Some("a".to_string()), 0, 1);
    let label_b = LabeledSpan::new(Some("b".to_string()), 5, 2);
    let label_c = LabeledSpan::new(Some("c".to_string()), 10, 3);

    let diag = MietteDiagnostic::new("replacement test")
        .with_label(label_a)
        .with_labels(vec![label_b, label_c]);

    assert_eq!(diag.message, "replacement test");
    let labels = diag.labels.as_ref().unwrap();

    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].label(), Some("b"));
    assert_eq!(labels[1].label(), Some("c"));
    let span0: &SourceSpan = labels[0].inner();
    assert_eq!(span0.offset(), 5);
    assert_eq!(span0.len(), 2);
    let span1: &SourceSpan = labels[1].inner();
    assert_eq!(span1.offset(), 10);
    assert_eq!(span1.len(), 3);
}

#[test]
fn test_miette_diagnostic_chaining_and_labels_after_and_label() {
    let diag = MietteDiagnostic::new("chaining test")
        .and_label(LabeledSpan::new(Some("first_and".to_string()), 0, 3))
        .and_label(LabeledSpan::new(Some("second_and".to_string()), 5, 4))
        .and_labels(vec![
            LabeledSpan::new(Some("batch1".to_string()), 15, 2),
            LabeledSpan::new(Some("batch2".to_string()), 20, 6),
        ]);

    assert_eq!(diag.message, "chaining test");
    let labels = diag.labels.as_ref().unwrap();
    assert_eq!(labels.len(), 4);
    assert_eq!(labels[0].label(), Some("first_and"));
    assert_eq!(labels[1].label(), Some("second_and"));
    assert_eq!(labels[2].label(), Some("batch1"));
    assert_eq!(labels[3].label(), Some("batch2"));
    let span_last: &SourceSpan = labels[3].inner();
    assert_eq!(span_last.offset(), 20);
    assert_eq!(span_last.len(), 6);
}

#[test]
fn test_miette_diagnostic_severity_clone() {
    let sev = Severity::Warning;
    let cloned = sev.clone();
    assert_eq!(sev, cloned);
    assert_eq!(cloned, Severity::Warning);

    let sev2 = Severity::Error;
    let cloned2 = sev2.clone();
    assert_eq!(sev2, cloned2);
    assert_ne!(cloned, cloned2);

    let sev3 = Severity::Advice;
    let cloned3 = sev3.clone();
    assert_eq!(sev3, cloned3);
    assert_ne!(cloned3, Severity::Error);
    assert_ne!(cloned3, Severity::Warning);
}

#[test]
fn test_miette_diagnostic_as_diagnostic_trait() {
    let diag = MietteDiagnostic::new("trait test")
        .with_code("T100")
        .with_severity(Severity::Warning)
        .with_help("try something else")
        .with_url("https://example.com/T100")
        .with_label(LabeledSpan::new(Some("here".to_string()), 3, 7));


    let d: &dyn Diagnostic = &diag;
    let code = d.code().unwrap();
    let code_str = format!("{}", code);
    assert_eq!(code_str, "T100");

    assert_eq!(d.severity(), Some(Severity::Warning));

    let help = d.help().unwrap();
    let help_str = format!("{}", help);
    assert_eq!(help_str, "try something else");

    let url = d.url().unwrap();
    let url_str = format!("{}", url);
    assert_eq!(url_str, "https://example.com/T100");

    let labels: Vec<_> = d.labels().unwrap().collect();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].label(), Some("here"));
    let inner: &SourceSpan = labels[0].inner();
    assert_eq!(inner.offset(), 3);
    assert_eq!(inner.len(), 7);
}