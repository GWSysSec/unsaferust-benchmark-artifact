use miette::{Diagnostic, NarratableReportHandler, ReportHandler};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
#[error("top-level error occurred")]
#[diagnostic(code(test::top_level), help("check the inner cause"))]
struct TopLevelError {
    #[source]
    inner: InnerError,
}

#[derive(Debug, Diagnostic, Error)]
#[error("inner error: something went wrong")]
#[diagnostic(code(test::inner))]
struct InnerError {
    #[source]
    root: RootCauseError,
}

#[derive(Debug, Diagnostic, Error)]
#[error("root cause: file not found")]
#[diagnostic(code(test::root_cause), severity(Error))]
struct RootCauseError;

#[derive(Debug, Diagnostic, Error)]
#[error("simple error with no cause")]
#[diagnostic(code(test::simple), help("nothing to do here"))]
struct SimpleError;

fn render_to_string(handler: &NarratableReportHandler, diag: &dyn Diagnostic) -> String {
    struct Wrapper<'a> {
        handler: &'a NarratableReportHandler,
        diag: &'a dyn Diagnostic,
    }
    impl<'a> fmt::Debug for Wrapper<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.handler.debug(self.diag, f)
        }
    }
    format!("{:?}", Wrapper { handler, diag })
}

#[test]
fn narratable_with_cause_chain_shows_causes() {
    let error = TopLevelError {
        inner: InnerError {
            root: RootCauseError,
        },
    };

    let handler = NarratableReportHandler::new().with_cause_chain();
    let output = render_to_string(&handler, &error);


    assert!(
        output.contains("top-level error occurred"),
        "Expected top-level message in output: {}",
        output
    );


    assert!(
        output.contains("inner error: something went wrong"),
        "Expected inner error message in output: {}",
        output
    );


    assert!(
        output.contains("root cause: file not found"),
        "Expected root cause message in output: {}",
        output
    );


    assert!(
        output.contains("test::top_level"),
        "Expected diagnostic code test::top_level in output: {}",
        output
    );


    assert!(
        output.contains("check the inner cause"),
        "Expected help text in output: {}",
        output
    );


    let cause_count = output.matches("Caused by").count();
    assert!(
        cause_count >= 1,
        "Expected at least 1 'Caused by' section, got {}: {}",
        cause_count,
        output
    );


    assert!(
        output.len() > 50,
        "Expected substantial output, got {} bytes",
        output.len()
    );


    let line_count = output.lines().count();
    assert!(
        line_count >= 3,
        "Expected at least 3 lines in narratable output, got {}",
        line_count
    );
}

#[test]
fn narratable_without_cause_chain_hides_causes() {
    let error = TopLevelError {
        inner: InnerError {
            root: RootCauseError,
        },
    };

    let handler = NarratableReportHandler::new().without_cause_chain();
    let output = render_to_string(&handler, &error);


    assert!(
        output.contains("top-level error occurred"),
        "Expected top-level message in output: {}",
        output
    );


    assert!(
        !output.contains("inner error: something went wrong"),
        "Did NOT expect inner error message in output when cause chain is disabled: {}",
        output
    );


    assert!(
        !output.contains("root cause: file not found"),
        "Did NOT expect root cause message in output when cause chain is disabled: {}",
        output
    );


    assert!(
        output.contains("test::top_level"),
        "Expected diagnostic code test::top_level in output: {}",
        output
    );


    assert!(
        output.contains("check the inner cause"),
        "Expected help text in output: {}",
        output
    );


    let cause_count = output.matches("Caused by").count();
    assert_eq!(
        cause_count, 0,
        "Expected 0 'Caused by' sections when cause chain is disabled, got {}: {}",
        cause_count, output
    );


    let handler_with = NarratableReportHandler::new().with_cause_chain();
    let output_with = render_to_string(&handler_with, &error);
    assert!(
        output.len() < output_with.len(),
        "Expected output without cause chain ({}) to be shorter than with cause chain ({})",
        output.len(),
        output_with.len()
    );


    assert!(
        !output.is_empty(),
        "Expected non-empty output even without cause chain"
    );
}

#[test]
fn narratable_with_cause_chain_is_default_behavior() {
    let error = TopLevelError {
        inner: InnerError {
            root: RootCauseError,
        },
    };


    let default_handler = NarratableReportHandler::new();
    let default_output = render_to_string(&default_handler, &error);


    let with_chain_handler = NarratableReportHandler::new().with_cause_chain();
    let with_chain_output = render_to_string(&with_chain_handler, &error);


    assert_eq!(
        default_output, with_chain_output,
        "Default handler and with_cause_chain handler should produce identical output"
    );


    assert!(
        default_output.contains("inner error"),
        "Default output should contain inner error: {}",
        default_output
    );
    assert!(
        with_chain_output.contains("inner error"),
        "With-chain output should contain inner error: {}",
        with_chain_output
    );


    assert!(
        default_output.contains("root cause"),
        "Default output should contain root cause: {}",
        default_output
    );
    assert!(
        with_chain_output.contains("root cause"),
        "With-chain output should contain root cause: {}",
        with_chain_output
    );


    assert_eq!(
        default_output.len(),
        with_chain_output.len(),
        "Outputs should have identical length"
    );


    assert_eq!(
        default_output.lines().count(),
        with_chain_output.lines().count(),
        "Outputs should have identical line count"
    );


    assert!(
        default_output.contains("Caused by"),
        "Default should contain 'Caused by'"
    );
}

#[test]
fn narratable_toggle_cause_chain_on_and_off() {
    let error = TopLevelError {
        inner: InnerError {
            root: RootCauseError,
        },
    };


    let handler_off = NarratableReportHandler::new()
        .with_cause_chain()
        .without_cause_chain();
    let output_off = render_to_string(&handler_off, &error);


    let handler_on = NarratableReportHandler::new()
        .without_cause_chain()
        .with_cause_chain();
    let output_on = render_to_string(&handler_on, &error);


    assert!(
        !output_off.contains("inner error: something went wrong"),
        "Toggled-off handler should not show inner error: {}",
        output_off
    );


    assert!(
        output_on.contains("inner error: something went wrong"),
        "Toggled-on handler should show inner error: {}",
        output_on
    );


    assert!(
        !output_off.contains("root cause: file not found"),
        "Toggled-off handler should not show root cause: {}",
        output_off
    );


    assert!(
        output_on.contains("root cause: file not found"),
        "Toggled-on handler should show root cause: {}",
        output_on
    );


    assert!(
        output_on.len() > output_off.len(),
        "On version ({}) should be longer than off version ({})",
        output_on.len(),
        output_off.len()
    );


    assert!(
        output_off.contains("top-level error occurred"),
        "Off version should still have top-level error"
    );
    assert!(
        output_on.contains("top-level error occurred"),
        "On version should still have top-level error"
    );


    assert_eq!(
        output_off.matches("Caused by").count(),
        0,
        "Off version should have 0 'Caused by'"
    );
    assert!(
        output_on.matches("Caused by").count() >= 1,
        "On version should have at least 1 'Caused by'"
    );
}

#[test]
fn narratable_without_cause_chain_simple_error_unchanged() {
    let error = SimpleError;

    let handler_with = NarratableReportHandler::new().with_cause_chain();
    let output_with = render_to_string(&handler_with, &error);

    let handler_without = NarratableReportHandler::new().without_cause_chain();
    let output_without = render_to_string(&handler_without, &error);


    assert_eq!(
        output_with, output_without,
        "For errors without causes, with/without cause chain should be identical"
    );


    assert!(
        output_with.contains("simple error with no cause"),
        "Should contain error message: {}",
        output_with
    );


    assert!(
        output_with.contains("test::simple"),
        "Should contain diagnostic code: {}",
        output_with
    );


    assert!(
        output_with.contains("nothing to do here"),
        "Should contain help text: {}",
        output_with
    );


    assert_eq!(
        output_with.matches("Caused by").count(),
        0,
        "Simple error should have no 'Caused by' with chain"
    );
    assert_eq!(
        output_without.matches("Caused by").count(),
        0,
        "Simple error should have no 'Caused by' without chain"
    );


    assert_eq!(output_with.len(), output_without.len());


    assert_eq!(output_with.lines().count(), output_without.lines().count());
}