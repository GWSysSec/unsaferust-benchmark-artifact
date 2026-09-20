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

    // The output should contain the top-level message
    assert!(
        output.contains("top-level error occurred"),
        "Expected top-level message in output: {}",
        output
    );

    // The output should contain the inner cause
    assert!(
        output.contains("inner error: something went wrong"),
        "Expected inner error message in output: {}",
        output
    );

    // The output should contain the root cause
    assert!(
        output.contains("root cause: file not found"),
        "Expected root cause message in output: {}",
        output
    );

    // The output should contain diagnostic codes
    assert!(
        output.contains("test::top_level"),
        "Expected diagnostic code test::top_level in output: {}",
        output
    );

    // Should contain help text
    assert!(
        output.contains("check the inner cause"),
        "Expected help text in output: {}",
        output
    );

    // Verify the cause chain includes multiple "Caused by" or similar indicators
    let cause_count = output.matches("Caused by").count();
    assert!(
        cause_count >= 1,
        "Expected at least 1 'Caused by' section, got {}: {}",
        cause_count,
        output
    );

    // Verify the output is non-empty and has substantial content
    assert!(
        output.len() > 50,
        "Expected substantial output, got {} bytes",
        output.len()
    );

    // Verify the output contains multiple lines (narratable format is multi-line)
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

    // The output should still contain the top-level message
    assert!(
        output.contains("top-level error occurred"),
        "Expected top-level message in output: {}",
        output
    );

    // The output should NOT contain the inner cause message
    assert!(
        !output.contains("inner error: something went wrong"),
        "Did NOT expect inner error message in output when cause chain is disabled: {}",
        output
    );

    // The output should NOT contain the root cause message
    assert!(
        !output.contains("root cause: file not found"),
        "Did NOT expect root cause message in output when cause chain is disabled: {}",
        output
    );

    // Should still contain the top-level diagnostic code
    assert!(
        output.contains("test::top_level"),
        "Expected diagnostic code test::top_level in output: {}",
        output
    );

    // Should still contain help text for the top-level diagnostic
    assert!(
        output.contains("check the inner cause"),
        "Expected help text in output: {}",
        output
    );

    // Should NOT have "Caused by" sections
    let cause_count = output.matches("Caused by").count();
    assert_eq!(
        cause_count, 0,
        "Expected 0 'Caused by' sections when cause chain is disabled, got {}: {}",
        cause_count, output
    );

    // The output without cause chain should be shorter than with cause chain
    let handler_with = NarratableReportHandler::new().with_cause_chain();
    let output_with = render_to_string(&handler_with, &error);
    assert!(
        output.len() < output_with.len(),
        "Expected output without cause chain ({}) to be shorter than with cause chain ({})",
        output.len(),
        output_with.len()
    );

    // Verify the output is still non-empty
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

    // Default handler
    let default_handler = NarratableReportHandler::new();
    let default_output = render_to_string(&default_handler, &error);

    // Explicitly with cause chain
    let with_chain_handler = NarratableReportHandler::new().with_cause_chain();
    let with_chain_output = render_to_string(&with_chain_handler, &error);

    // They should produce identical output
    assert_eq!(
        default_output, with_chain_output,
        "Default handler and with_cause_chain handler should produce identical output"
    );

    // Both should contain cause information
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

    // Both should contain root cause
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

    // Verify lengths are equal
    assert_eq!(
        default_output.len(),
        with_chain_output.len(),
        "Outputs should have identical length"
    );

    // Verify line counts are equal
    assert_eq!(
        default_output.lines().count(),
        with_chain_output.lines().count(),
        "Outputs should have identical line count"
    );

    // Verify both contain Caused by
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

    // Start with cause chain, then disable it
    let handler_off = NarratableReportHandler::new()
        .with_cause_chain()
        .without_cause_chain();
    let output_off = render_to_string(&handler_off, &error);

    // Start without cause chain, then enable it
    let handler_on = NarratableReportHandler::new()
        .without_cause_chain()
        .with_cause_chain();
    let output_on = render_to_string(&handler_on, &error);

    // The "off" version should not have causes
    assert!(
        !output_off.contains("inner error: something went wrong"),
        "Toggled-off handler should not show inner error: {}",
        output_off
    );

    // The "on" version should have causes
    assert!(
        output_on.contains("inner error: something went wrong"),
        "Toggled-on handler should show inner error: {}",
        output_on
    );

    // The "off" version should not have root cause
    assert!(
        !output_off.contains("root cause: file not found"),
        "Toggled-off handler should not show root cause: {}",
        output_off
    );

    // The "on" version should have root cause
    assert!(
        output_on.contains("root cause: file not found"),
        "Toggled-on handler should show root cause: {}",
        output_on
    );

    // The on version should be longer
    assert!(
        output_on.len() > output_off.len(),
        "On version ({}) should be longer than off version ({})",
        output_on.len(),
        output_off.len()
    );

    // Both should still have the top-level error
    assert!(
        output_off.contains("top-level error occurred"),
        "Off version should still have top-level error"
    );
    assert!(
        output_on.contains("top-level error occurred"),
        "On version should still have top-level error"
    );

    // Verify cause counts
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

    // For an error with no causes, both should produce the same output
    assert_eq!(
        output_with, output_without,
        "For errors without causes, with/without cause chain should be identical"
    );

    // Both should contain the error message
    assert!(
        output_with.contains("simple error with no cause"),
        "Should contain error message: {}",
        output_with
    );

    // Both should contain the diagnostic code
    assert!(
        output_with.contains("test::simple"),
        "Should contain diagnostic code: {}",
        output_with
    );

    // Both should contain help text
    assert!(
        output_with.contains("nothing to do here"),
        "Should contain help text: {}",
        output_with
    );

    // Neither should have "Caused by"
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

    // Lengths should be equal
    assert_eq!(output_with.len(), output_without.len());

    // Line counts should be equal
    assert_eq!(output_with.lines().count(), output_without.lines().count());
}