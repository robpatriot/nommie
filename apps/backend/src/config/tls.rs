//! TLS certificate expiry checking utilities

use std::path::Path;

use tracing::warn;

/// Check certificate expiry and log a warning if expiring soon.
///
/// This function reads the certificate file at the given path, extracts the
/// expiry date using `openssl`, and logs a warning if the certificate expires
/// within the specified number of days.
///
/// # Arguments
///
/// * `cert_path` - Path to the certificate file (PEM format)
/// * `warn_days` - Number of days before expiry to start warning (default: 90)
///
/// # Returns
///
/// Returns `Ok(())` if the check completes (even if a warning was logged),
/// or an error if the certificate file cannot be read or parsed.
pub fn check_cert_expiry(cert_path: &str, warn_days: u64) -> Result<(), String> {
    let path = Path::new(cert_path);

    if !path.exists() {
        // Don't error if cert path doesn't exist (might be disabled)
        return Ok(());
    }

    // Use openssl command to extract expiry date
    let output = std::process::Command::new("openssl")
        .arg("x509")
        .arg("-enddate")
        .arg("-noout")
        .arg("-in")
        .arg(cert_path)
        .output()
        .map_err(|e| format!("Failed to run openssl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("openssl failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse output like "notAfter=Dec 31 23:59:59 2026 GMT"
    let expiry_str = stdout
        .trim()
        .strip_prefix("notAfter=")
        .ok_or_else(|| format!("Unexpected openssl output format: {}", stdout))?;

    // Calculate days until expiry using date command (works on Unix-like systems)
    // We'll use a simple approach: convert to epoch seconds and compare
    let expiry_epoch = std::process::Command::new("date")
        .arg("+%s")
        .arg("-d")
        .arg(expiry_str)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<i64>()
                    .ok()
            } else {
                None
            }
        });

    let now_epoch = std::process::Command::new("date")
        .arg("+%s")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<i64>()
                    .ok()
            } else {
                None
            }
        });

    match (expiry_epoch, now_epoch) {
        (Some(expiry), Some(now)) => {
            let days_until_expiry = ((expiry - now) / 86400) as u64;

            if days_until_expiry <= warn_days {
                warn!(
                    tls_cert_expiring_soon = true,
                    cert_path = %cert_path,
                    days_until_expiry = days_until_expiry,
                    expiry_date = %expiry_str,
                    "TLS certificate is expiring soon. Consider rotating certificates. See docker/postgres-tls/README.md for rotation instructions."
                );
            }
        }
        _ => {
            // If date parsing fails, just log the expiry string as a fallback
            warn!(
                tls_cert_expiry_check = "partial",
                cert_path = %cert_path,
                expiry_date = %expiry_str,
                "Certificate expiry date extracted. Please verify manually if expiring within {} days.",
                warn_days
            );
        }
    }

    Ok(())
}

/// Check Postgres TLS certificate expiry if configured.
///
/// This is a convenience function that checks the certificate specified by
/// `POSTGRES_SSL_ROOT_CERT` environment variable. It logs warnings but does
/// not fail startup if the check fails.
pub fn check_postgres_cert_expiry() {
    if let Ok(cert_path) = std::env::var("POSTGRES_SSL_ROOT_CERT") {
        if let Err(e) = check_cert_expiry(&cert_path, 90) {
            // Log error but don't fail startup
            warn!(
                tls_cert_check_failed = true,
                error = %e,
                "Failed to check TLS certificate expiry"
            );
        }
    }
}
