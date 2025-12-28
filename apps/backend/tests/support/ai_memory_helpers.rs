//! Test helpers for AI memory mode conversions

use backend::ai::memory::MemoryMode;

/// Convert MemoryMode to database value for testing.
///
/// This helper is used in tests to verify MemoryMode conversion logic.
/// Production code stores memory_level as `Option<i32>` directly (not as MemoryMode enum),
/// so this conversion is only needed for test assertions.
pub fn memory_mode_to_db_value(mode: MemoryMode) -> Option<i32> {
    match mode {
        MemoryMode::Full => Some(100),
        MemoryMode::Partial { level } => Some(level),
        MemoryMode::None => Some(0),
    }
}
