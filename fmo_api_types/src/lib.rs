use bitcoin::address::NetworkUnchecked;
use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FedimintTotals {
    pub federations: u64,
    pub tx_volume: Amount,
    pub tx_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSummary {
    pub id: FederationId,
    pub name: Option<String>,
    pub last_7d_activity: Vec<FederationActivity>,
    pub deposits: Amount,
    pub invite: String,
    pub nostr_votes: FederationRating,
    pub health: FederationHealth,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FederationRating {
    pub count: u64,
    pub avg: Option<f64>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct FederationActivity {
    pub num_transactions: u64,
    pub amount_transferred: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationUtxo {
    pub address: bitcoin::Address<NetworkUnchecked>,
    pub out_point: bitcoin::OutPoint,
    pub amount: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianHealth {
    pub avg_uptime: f32,
    pub avg_latency: f32,
    pub latest: Option<GuardianHealthLatest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianHealthLatest {
    pub block_height: u32,
    pub block_outdated: bool,
    pub session_count: u32,
    pub session_outdated: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FederationHealth {
    Online,
    Degraded,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoncesRequest {
    pub nonces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceSpendInfo {
    pub session_index: u64,
    pub estimated_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}
