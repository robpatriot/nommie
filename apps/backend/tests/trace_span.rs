//! Integration test for handler logs inheriting `trace_id` via a request span.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use actix_web::{test, web, App, HttpResponse};
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::structured_logger::StructuredLogger;
use serde_json::Value;
use serial_test::serial;
use tracing::info;
use tracing::subscriber::set_global_default;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::Registry;

/// Simple writer that appends JSON lines to a shared Vec<u8>.
#[derive(Clone)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut guard = self.0.lock().unwrap();
        guard.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[actix_web::test]
#[serial]
async fn handler_logs_are_in_request_span_with_trace_id() {
    // ---- 1) In-memory JSON logger that records span list, globally (worker threads) ----
    let buf = Arc::new(Mutex::new(Vec::new()));
    let make_writer = {
        let buf = buf.clone();
        move || BufWriter(buf.clone())
    };

    let subscriber = Registry::default().with(
        fmt::Layer::default()
            .json()
            .with_span_list(true) // include active spans list on each event
            .with_current_span(true) // include the current span object
            .with_ansi(false)
            .with_writer(make_writer),
    );
    set_global_default(subscriber).expect("set global subscriber");

    // ---- 2) Minimal app ----
    let app = test::init_service(
        App::new()
            .wrap(StructuredLogger)
            .wrap(backend::middleware::trace_span::TraceSpan)
            .wrap(RequestTrace)
            .route(
                "/ping",
                web::get().to(|| async {
                    info!("inside handler");
                    Ok::<HttpResponse, backend::AppError>(HttpResponse::Ok().finish())
                }),
            ),
    )
    .await;

    // ---- 3) Perform request ----
    let req = test::TestRequest::get().uri("/ping").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Capture trace id from header for comparison with span field.
    let trace_id = resp
        .headers()
        .get("X-Trace-Id")
        .expect("X-Trace-Id header present")
        .to_str()
        .unwrap()
        .to_string();

    // ---- 4) Parse captured JSON lines and look for the handler event ----
    let data = {
        let bytes = buf.lock().unwrap().clone();
        String::from_utf8(bytes).expect("utf8")
    };

    let mut saw_inside = false;
    let mut saw_trace_match = false;

    for line in data.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue; // ignore any non-JSON noise
        };

        // Extract the message field from "fields.message"
        let msg = v
            .get("fields")
            .and_then(|f| f.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("");

        if msg.contains("inside handler") {
            saw_inside = true;

            // Look for a span named "request" with trace_id == header value.
            if let Some(spans) = v.get("spans").and_then(Value::as_array) {
                for s in spans {
                    if s.get("name").and_then(Value::as_str) == Some("request") {
                        // Directly check trace_id in the span object
                        if let Some(tid) = s.get("trace_id").and_then(Value::as_str) {
                            if tid == trace_id {
                                saw_trace_match = true;
                            }
                        }
                    }
                }
            }
        }
    }

    // We should at least have captured the handler log line.
    assert!(saw_inside, "did not capture handler log line");

    assert!(
        saw_trace_match,
        "handler log did not inherit trace_id via active request span"
    );
}
