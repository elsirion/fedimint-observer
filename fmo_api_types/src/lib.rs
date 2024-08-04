use fedimint_core::config::FederationId;
use fedimint_core::Amount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSummary {
    pub id: FederationId,
    pub name: Option<String>,
    pub last_7d_activity: Vec<FederationActivity>,
    pub deposits: Amount,
    pub invite: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationActivity {
    pub num_transactions: u64,
    pub amount_transferred: Amount,
}
