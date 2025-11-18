use tokio::sync::broadcast;
use anyhow::Result;
use tracing::{info, error, warn};
use futures_util::StreamExt;
use redis::aio::PubSub;

use crate::types::DIDClaimedEvent;

pub struct RedisSubscriber {
    tx: broadcast::Sender<DIDClaimedEvent>,
}

impl RedisSubscriber {
    pub fn new(tx: broadcast::Sender<DIDClaimedEvent>) -> Self {
        Self { tx }
    }

    pub async fn start(&self, redis_url: String) -> Result<()> {
        let tx_clone = self.tx.clone();
        
        tokio::spawn(async move {
            info!("🔌 Connecting to Redis...");
            
            let client = redis::Client::open(redis_url).unwrap();
            let con = client.get_async_connection().await.unwrap();
            let mut pubsub: PubSub = con.into_pubsub();
            
            // SUBSCRIBE to the channel
            pubsub.subscribe("did_claimed").await.unwrap();
            info!("✅ Subscribed to Redis channel: did_claimed");
            
            let mut stream = pubsub.on_message();
            
            info!("👂 Listening for DID claim events from Redis...");
            
            // Listen for messages
            while let Some(msg) = stream.next().await {
                let payload: String = msg.get_payload().unwrap();
                
                info!("📬 Received event from Redis Pub/Sub");
                
                match serde_json::from_str::<DIDClaimedEvent>(&payload) {
                    Ok(event) => {
                        info!("📤 Broadcasting to {} SSE clients", tx_clone.receiver_count());
                        
                        match tx_clone.send(event) {
                            Ok(n) => info!("✅ Pushed to {} client(s)", n),
                            Err(_) => warn!("⚠️  No SSE clients connected"),
                        }
                    }
                    Err(e) => error!("❌ Failed to parse event: {}", e),
                }
            }
        });
        
        Ok(())
    }
}
