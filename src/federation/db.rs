use std::str::FromStr;

use fedimint_core::config::{ClientConfig, FederationId};
use sqlx::any::AnyRow;
use sqlx::{Error, Row};

pub struct Federation {
    pub federation_id: FederationId,
    pub config: ClientConfig,
}

impl sqlx::FromRow<'_, AnyRow> for Federation {
    fn from_row(row: &AnyRow) -> Result<Self, Error> {
        let federation_id_str: String = row.try_get("federation_id")?;
        let federation_id =
            FederationId::from_str(&federation_id_str).map_err(|e| Error::Decode(e.into()))?;

        let config_str: String = row.try_get("config")?;
        let config = serde_json::from_str(&config_str).map_err(|e| Error::Decode(e.into()))?;

        Ok(Federation {
            federation_id,
            config,
        })
    }
}
