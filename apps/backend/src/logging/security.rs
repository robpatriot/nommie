use tracing::warn;

use crate::logging::pii::Redacted;
use crate::web::trace_ctx;

/// Log a security-relevant login failure event.
pub fn login_failed(reason: &str, email: Option<&str>) {
    let trace_id = trace_ctx::trace_id();

    warn!(
        event = "SECURITY_LOGIN_FAILED",
        %trace_id,
        email = %email.map(Redacted).unwrap_or(Redacted("")),
        reason,
        "Authentication failure"
    );
}

/// Log a security-relevant rate-limit event.
pub fn rate_limit_hit(endpoint: &str) {
    let trace_id = trace_ctx::trace_id();

    warn!(
        event = "SECURITY_RATE_LIMIT_HIT",
        %trace_id,
        endpoint,
        "Rate limit exceeded"
    );
}
