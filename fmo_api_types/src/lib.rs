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

/// Subset of a gateway's registration info suitable for public API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayInfo {
    /// Gateway's public key (hex-encoded)
    pub gateway_id: String,
    /// LN node public key (hex-encoded)
    pub node_pub_key: String,
    pub lightning_alias: String,
    /// URL of the gateway's public API
    pub api_endpoint: String,
    /// Whether the federation has vetted this gateway
    pub vetted: bool,
    /// Full raw announcement, useful for forwards-compatible client usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}
