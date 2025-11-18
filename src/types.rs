use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Event sent to SSE clients
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DIDClaimedEvent {
    pub registry_id: String,
    pub user_address: String,
    pub did_type: i16,
    pub user_did_id: String,
    pub nft_id: String,
    pub checkpoint_sequence_number: i64,
    pub transaction_digest: String,
    pub timestamp_ms: i64,
    pub event_index: i64,
}
