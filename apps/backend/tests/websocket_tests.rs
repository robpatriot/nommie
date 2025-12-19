// WebSocket and realtime sync tests
//
// Tests for WebSocket connections, broadcasts, reconnection, and shutdown behavior.
//
// Run all websocket tests:
//   cargo test --test websocket_tests
//
// Run specific websocket tests:
//   cargo test --test websocket_tests websocket::connection_tests::

mod common;
mod support;

#[path = "suites/websocket/mod.rs"]
mod websocket;
