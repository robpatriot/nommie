// apps/backend/tests/support/websocket_client.rs
// WebSocket client utilities for testing (protocol-aware)

use std::error::Error;
use std::fmt;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

#[derive(Debug)]
struct WsTestError(String);

impl fmt::Display for WsTestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for WsTestError {}

fn err(msg: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(WsTestError(msg.into()))
}

/// WebSocket test client
pub struct WebSocketClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocketClient {
    /// Connect to a WebSocket endpoint
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, _) = connect_async(url).await?;
        Ok(Self { stream })
    }

    /// Connect to a WebSocket endpoint, retrying until success or timeout.
    pub async fn connect_retry(
        url: &str,
        timeout: Duration,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let start = tokio::time::Instant::now();

        loop {
            match connect_async(url).await {
                Ok((stream, _)) => return Ok(Self { stream }),
                Err(e) => {
                    if start.elapsed() >= timeout {
                        return Err(Box::new(e));
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        }
    }

    /// Receive the next tungstenite message with a timeout.
    pub async fn recv_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        tokio::time::timeout(timeout, self.stream.next())
            .await
            .map_err(|_| err(format!("Timeout waiting for message after {:?}", timeout)))?
            .transpose()
            .map_err(|e| e.into())
    }

    /// Send a raw text message
    pub async fn send(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(Message::Text(text.into())).await?;
        Ok(())
    }

    /// Send a JSON message
    pub async fn send_json(&mut self, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let text = serde_json::to_string(value)?;
        self.send(&text).await
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.close(None).await?;
        Ok(())
    }

    /// Receive next text/binary frame and parse as JSON, but:
    /// - FAIL FAST on {"type":"error", ...} by returning Err(...)
    /// - Return Ok(None) on timeout or close (useful for "expect no message" tests)
    pub async fn recv_json_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Value>, Box<dyn std::error::Error>> {
        let msg = match tokio::time::timeout(timeout, self.stream.next()).await {
            Ok(opt) => opt,
            Err(_) => return Ok(None), // timeout => no message
        };

        let msg = match msg {
            Some(Ok(m)) => m,
            Some(Err(e)) => return Err(e.into()),
            None => return Ok(None), // EOF
        };

        match msg {
            Message::Text(text) => {
                let v: Value = serde_json::from_str(&text)
                    .map_err(|e| err(format!("Invalid JSON from server: {e}; raw={text}")))?;

                if v.get("type").and_then(|t| t.as_str()) == Some("error") {
                    let code = v.get("code").cloned().unwrap_or(Value::Null);
                    let message = v.get("message").cloned().unwrap_or(Value::Null);
                    return Err(err(format!(
                        "Server sent error message: code={code} message={message} full={v}"
                    )));
                }

                Ok(Some(v))
            }

            Message::Binary(bin) => {
                let v: Value = serde_json::from_slice(&bin)
                    .map_err(|e| err(format!("Invalid JSON (binary) from server: {e}")))?;

                if v.get("type").and_then(|t| t.as_str()) == Some("error") {
                    let code = v.get("code").cloned().unwrap_or(Value::Null);
                    let message = v.get("message").cloned().unwrap_or(Value::Null);
                    return Err(err(format!(
                        "Server sent error message: code={code} message={message} full={v}"
                    )));
                }

                Ok(Some(v))
            }

            Message::Close(_) => Ok(None),

            // Ignore ping/pong/other frames as "no JSON message"
            Message::Ping(_) | Message::Pong(_) => Ok(None),
            _ => Ok(None),
        }
    }

    /// Receive next text/binary frame and parse as JSON.
    /// Returns Ok(None) on timeout/close.
    /// Unlike recv_json_timeout, this does NOT fail-fast on {"type":"error"}.
    /// Use this for tests that specifically assert error-handling behavior.
    pub async fn recv_json_allow_error(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Value>, Box<dyn std::error::Error>> {
        let msg = match tokio::time::timeout(timeout, self.stream.next()).await {
            Ok(opt) => opt,
            Err(_) => return Ok(None), // timeout => no message
        };

        let msg = match msg {
            Some(Ok(m)) => m,
            Some(Err(e)) => return Err(e.into()),
            None => return Ok(None), // EOF
        };

        match msg {
            Message::Text(text) => Ok(Some(serde_json::from_str(&text)?)),
            Message::Binary(bin) => Ok(Some(serde_json::from_slice(&bin)?)),
            Message::Close(_) => Ok(None),
            Message::Ping(_) | Message::Pong(_) => Ok(None),
            _ => Ok(None),
        }
    }

    /// Receive messages until one with `type == expected_type` arrives (or timeout).
    /// Fails fast if the server sends a protocol error message.
    pub async fn recv_type(
        &mut self,
        timeout: Duration,
        expected_type: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let start = tokio::time::Instant::now();
        loop {
            let remaining = timeout.checked_sub(start.elapsed()).ok_or_else(|| {
                err(format!(
                    "Timeout waiting for message type '{expected_type}'"
                ))
            })?;

            match self.recv_json_timeout(remaining).await? {
                Some(msg) => {
                    if msg.get("type").and_then(|v| v.as_str()) == Some(expected_type) {
                        return Ok(msg);
                    }
                }
                None => continue,
            }
        }
    }

    /// Like recv_type, but DOES NOT fail-fast on {"type":"error"}.
    /// Use this only in tests that explicitly assert error-handling behavior.
    pub async fn recv_type_allow_error(
        &mut self,
        timeout: Duration,
        expected_type: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let start = tokio::time::Instant::now();
        loop {
            let remaining = timeout.checked_sub(start.elapsed()).ok_or_else(|| {
                err(format!(
                    "Timeout waiting for message type '{expected_type}'"
                ))
            })?;

            match self.recv_json_allow_error(remaining).await? {
                Some(msg) => {
                    if msg.get("type").and_then(|v| v.as_str()) == Some(expected_type) {
                        return Ok(msg);
                    }
                }
                None => continue,
            }
        }
    }

    /// Protocol helper: send hello + await hello_ack.
    pub async fn hello(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        self.send_json(&json!({ "type": "hello", "protocol": 1 }))
            .await?;
        self.recv_type(Duration::from_secs(5), "hello_ack").await
    }

    /// Protocol helper: subscribe to a game, enforcing ordering: ack then game_state.
    pub async fn subscribe_game(
        &mut self,
        game_id: i64,
    ) -> Result<(Value, Value), Box<dyn std::error::Error>> {
        self.send_json(&json!({
            "type": "subscribe",
            "topic": { "kind": "game", "id": game_id }
        }))
        .await?;

        let ack = self.recv_type(Duration::from_secs(5), "ack").await?;
        let game_state = self.recv_type(Duration::from_secs(5), "game_state").await?;
        Ok((ack, game_state))
    }
}
