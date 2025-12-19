// WebSocket client utilities for testing

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

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
                Err(err) => {
                    if start.elapsed() >= timeout {
                        return Err(Box::new(err));
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        }
    }

    /// Receive the next message with a timeout
    pub async fn recv_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        tokio::time::timeout(timeout, self.stream.next())
            .await
            .map_err(|_| "Timeout waiting for message")?
            .transpose()
            .map_err(|e| e.into())
    }

    /// Receive the next message (waits indefinitely)
    pub async fn recv(&mut self) -> Option<Result<Message, tokio_tungstenite::tungstenite::Error>> {
        self.stream.next().await
    }

    /// Send a text message
    pub async fn send(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(Message::Text(text.to_string())).await?;
        Ok(())
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.close(None).await?;
        Ok(())
    }

    /// Parse next text message as JSON
    pub async fn recv_json_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<Value>, Box<dyn std::error::Error>> {
        match self.recv_timeout(timeout).await? {
            Some(Message::Text(text)) => {
                let json: Value = serde_json::from_str(&text)?;
                Ok(Some(json))
            }
            Some(Message::Close(_)) => Ok(None),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    }
}
