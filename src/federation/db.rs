use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::encoding::Decodable;
use sqlx::any::AnyRow;
use sqlx::{Error, Row};

pub struct Federation {
    pub federation_id: FederationId,
    pub config: ClientConfig,
}

impl sqlx::FromRow<'_, AnyRow> for Federation {
    fn from_row(row: &AnyRow) -> Result<Self, Error> {
        let federation_id_bytes: Vec<u8> = row.try_get("federation_id")?;
        let federation_id =
            FederationId::consensus_decode_vec(federation_id_bytes, &Default::default())
                .map_err(|e| Error::Decode(e.into()))?;

        let config_bytes: Vec<u8> = row.try_get("config")?;
        let config = ClientConfig::consensus_decode_vec(config_bytes, &Default::default())
            .map_err(|e| Error::Decode(e.into()))?;

        Ok(Federation {
            federation_id,
            config,
        })
    }
}
