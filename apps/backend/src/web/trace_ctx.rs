//! Task-local trace context for web requests.
//!
//! This module provides a minimal API for accessing the current request's trace_id
//! from anywhere in the request processing pipeline. It uses Tokio's task-local
//! storage to maintain the trace_id throughout the request lifecycle.
//!
//! This module is part of the web boundary and should not be imported by
//! core/service code to maintain separation of concerns.

use std::cell::RefCell;

use tokio::task_local;

task_local! {
    static TRACE_ID: RefCell<Option<String>>;
}

/// Get the trace_id for the current task.
/// Returns "unknown" if no trace_id is set (e.g., outside of a request context).
pub fn trace_id() -> String {
    TRACE_ID
        .try_with(|cell| {
            cell.borrow()
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string())
        })
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Run a future within a trace context.
/// This is used by middleware to establish the task-local scope.
pub async fn with_trace_id<F, R>(trace_id: String, future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    TRACE_ID.scope(RefCell::new(Some(trace_id)), future).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trace_id_outside_context() {
        // Outside of a trace context, should return "unknown"
        assert_eq!(trace_id(), "unknown");
    }

    #[tokio::test]
    async fn test_trace_id_within_context() {
        let test_trace_id = "test-trace-123".to_string();

        let result = with_trace_id(test_trace_id.clone(), async {
            // Within the trace context, should return the set trace_id
            assert_eq!(trace_id(), test_trace_id);
            "success"
        })
        .await;

        assert_eq!(result, "success");

        // After the context, should return "unknown" again
        assert_eq!(trace_id(), "unknown");
    }

    #[tokio::test]
    async fn test_nested_trace_contexts() {
        let outer_trace_id = "outer-trace-123".to_string();
        let inner_trace_id = "inner-trace-456".to_string();

        let result = with_trace_id(outer_trace_id.clone(), async {
            assert_eq!(trace_id(), outer_trace_id);

            let inner_result = with_trace_id(inner_trace_id.clone(), async {
                assert_eq!(trace_id(), inner_trace_id);
                "inner"
            })
            .await;

            // Should still be the outer trace_id
            assert_eq!(trace_id(), outer_trace_id);
            inner_result
        })
        .await;

        assert_eq!(result, "inner");
        assert_eq!(trace_id(), "unknown");
    }
}
