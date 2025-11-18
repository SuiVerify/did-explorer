use axum::{
    extract::{State, Query},
    response::sse::{Event, Sse},
    Json,
};
use futures_util::stream::Stream;
use std::convert::Infallible;
use tokio::sync::broadcast;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{info, error};

use crate::types::DIDClaimedEvent;

// SSE endpoint - for real-time push from Redis
pub async fn sse_events(
    State(rx): State<broadcast::Sender<DIDClaimedEvent>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut receiver = rx.subscribe();
    
    let stream = async_stream::stream! {
        info!("🔗 New SSE client connected");
        
        while let Ok(event) = receiver.recv().await {
            let json = serde_json::to_string(&event).unwrap();
            yield Ok(Event::default().data(json));
        }
        
        info!("👋 SSE client disconnected");
    };
    
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keepalive")
    )
}

// REST endpoint - for historical queries from PostgreSQL
#[derive(Deserialize)]
pub struct EventQuery {
    pub limit: Option<i64>,
    pub user_address: Option<String>,
}

pub async fn get_recent_events(
    State(db): State<PgPool>,
    Query(params): Query<EventQuery>,
) -> Json<Vec<DIDClaimedEvent>> {
    let limit = params.limit.unwrap_or(20);
    
    let query_result = if let Some(user_addr) = params.user_address {
        sqlx::query_as!(
            DIDClaimedEvent,
            r#"
            SELECT 
                registry_id, user_address, did_type, user_did_id, nft_id,
                checkpoint_sequence_number,
                transaction_digest,
                timestamp_ms,
                event_index
            FROM did_claimed_events
            WHERE user_address = $1
            ORDER BY timestamp_ms DESC
            LIMIT $2
            "#,
            user_addr,
            limit
        )
        .fetch_all(&db)
        .await
    } else {
        sqlx::query_as!(
            DIDClaimedEvent,
            r#"
            SELECT 
                registry_id, user_address, did_type, user_did_id, nft_id,
                checkpoint_sequence_number,
                transaction_digest,
                timestamp_ms,
                event_index
            FROM did_claimed_events
            ORDER BY timestamp_ms DESC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&db)
        .await
    };
    
    let events = match query_result {
        Ok(events) => events,
        Err(e) => {
            error!("Failed to query events: {}", e);
            Vec::new()
        }
    };
    
    Json(events)
}
