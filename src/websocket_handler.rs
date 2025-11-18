use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use anyhow::Result;
use tracing::{info, error};

use crate::types::{DIDClaimedEvent, WebSocketMessage};

/// Handle a single WebSocket connection
pub async fn handle_websocket(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    mut event_rx: broadcast::Receiver<DIDClaimedEvent>,
) -> Result<()> {
    let (mut write, mut read) = ws_stream.split();
    
    info!("🔗 New WebSocket client connected");

    // Spawn task to receive from PostgreSQL and send to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = WebSocketMessage::DIDClaimed { event };
            
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if write.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                }
            }
        }
    });

    // Receive messages from client (ping/pong, etc.)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                Message::Text(text) => {
                    info!("📩 Received from client: {}", text);
                }
                Message::Close(_) => {
                    info!("👋 Client disconnected");
                    break;
                }
                Message::Ping(_data) => {  // Fixed: added underscore
                    // Respond with pong (handled automatically)
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    info!("🔌 Client disconnected");
    Ok(())
}