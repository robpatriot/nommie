// Trace span integration tests
//
// This test binary does NOT import mod common because these tests need to
// set their own global tracing subscriber for testing purposes.
//
// Run these tests:
//   cargo test --test trace_span_tests

// NOTE: Do NOT add `mod common;` here - these tests set their own global subscriber

mod support;

#[path = "suites/routes/trace_span.rs"]
mod trace_span;
