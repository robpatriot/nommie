// Proptest prelude — shared configuration for integration tests.
//
// Env knobs:
// - PROPTEST_CASES: number of cases per property (e.g. 32, 800, 5000).
// - PROPTEST_MAX_SHRINK_MS: optional cap for shrinking time in milliseconds.
//
// To verify zero rejections (100% acceptance rate):
// - Run with high case count: PROPTEST_CASES=5000 pnpm be:test
// - If tests pass, reject count is < max_global_rejects (1024)
// - If tests fail with "Too many global rejects", optimization needed
// - Note: Proptest only reports reject counts on failure
//
// Best practices:
// - Avoid prop_assume! - use dependent generators instead
// - Generate valid inputs by construction, not by filtering

pub fn proptest_prelude_config() -> proptest::prelude::ProptestConfig {
    // Start from a single base default to avoid repeated default() calls
    let base: proptest::prelude::ProptestConfig = proptest::prelude::ProptestConfig::default();

    // PROPTEST_CASES: number of generated cases (default 8 for this project)
    let cases_env: Option<u32> = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok());
    // Fallback to our project default when missing/invalid, then clamp to at least 1
    let cases: u32 = cases_env.unwrap_or(8).max(1);

    // PROPTEST_MAX_SHRINK_MS: cap shrinking time in milliseconds (falls back to base.max_shrink_time)
    let max_shrink_time_env: Option<u32> = std::env::var("PROPTEST_MAX_SHRINK_MS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok());
    let max_shrink_time: u32 = max_shrink_time_env.unwrap_or(base.max_shrink_time);

    proptest::prelude::ProptestConfig {
        // Disable persistence to silence regression-file warnings in integration tests
        failure_persistence: None,
        cases,
        max_shrink_time,
        ..base
    }
}
