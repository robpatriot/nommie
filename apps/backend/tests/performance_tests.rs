// Performance tests
//
// Tests for database performance, query optimization, and benchmark comparisons.
//
// Run all performance tests:
//   cargo test --test performance_tests
//
// Run specific performance tests:
//   cargo test --test performance_tests performance::sqlite_performance::

mod common;
mod support;

#[path = "suites/performance/mod.rs"]
mod performance;
