use std::path::Path;

use tracing::warn;

pub fn check_cert_expiry(cert_path: &str, warn_days: u64) -> Result<(), String> {
    let path = Path::new(cert_path);

    if !path.exists() {
        return Ok(());
    }

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
    let expiry_str = stdout
        .trim()
        .strip_prefix("notAfter=")
        .ok_or_else(|| format!("Unexpected openssl output format: {}", stdout))?;

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

pub fn check_postgres_cert_expiry() {
    if let Ok(cert_path) = std::env::var("POSTGRES_SSL_ROOT_CERT") {
        if let Err(e) = check_cert_expiry(&cert_path, 90) {
            warn!(
                tls_cert_check_failed = true,
                error = %e,
                "Failed to check TLS certificate expiry"
            );
        }
    }
}
