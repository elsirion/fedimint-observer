use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::encoding::Decodable;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::TransactionId;
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

pub struct Transaction {
    pub txid: TransactionId,
    pub session_index: u64,
    pub item_index: u64,
    pub data: fedimint_core::transaction::Transaction,
}

impl sqlx::FromRow<'_, AnyRow> for crate::federation::db::Transaction {
    fn from_row(row: &AnyRow) -> Result<Self, Error> {
        let decoder = ModuleDecoderRegistry::default().with_fallback();

        let txid_bytes: Vec<u8> = row.try_get("txid")?;
        let txid = TransactionId::consensus_decode_vec(txid_bytes, &decoder)
            .map_err(|e| Error::Decode(e.into()))?;

        let session_index = row.try_get::<i64, _>("session_index")? as u64;

        let item_index = row.try_get::<i64, _>("item_index")? as u64;

        let data_bytes: Vec<u8> = row.try_get("data")?;
        let data =
            fedimint_core::transaction::Transaction::consensus_decode_vec(data_bytes, &decoder)
                .map_err(|e| Error::Decode(e.into()))?;

        Ok(crate::federation::db::Transaction {
            txid,
            session_index,
            item_index,
            data,
        })
    }
}
