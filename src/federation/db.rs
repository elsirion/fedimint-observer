use fedimint_core::config::{ClientConfig, FederationId};
use fedimint_core::encoding::Decodable;
use fedimint_core::module::registry::ModuleDecoderRegistry;
use fedimint_core::TransactionId;
use postgres_from_row::FromRow;
use tokio_postgres::{Error, Row};

pub struct Federation {
    pub federation_id: FederationId,
    pub config: ClientConfig,
}

impl FromRow for Federation {
    fn from_row(row: &Row) -> Self {
        Self::try_from_row(row).expect("Decoding row failed")
    }

    fn try_from_row(row: &Row) -> Result<Self, Error> {
        let federation_id_bytes: Vec<u8> = row.try_get("federation_id")?;
        let federation_id =
            FederationId::consensus_decode_vec(federation_id_bytes, &Default::default())
                .expect("Invalid data in DB");

        let config_bytes: Vec<u8> = row.try_get("config")?;
        let config = ClientConfig::consensus_decode_vec(config_bytes, &Default::default())
            .expect("Invalid data in DB");

        Ok(Federation {
            federation_id,
            config,
        })
    }
}

pub struct Transaction {
    pub txid: TransactionId,
    pub session_index: i32,
    pub item_index: i32,
    pub data: fedimint_core::transaction::Transaction,
}

impl FromRow for crate::federation::db::Transaction {
    fn from_row(row: &Row) -> Self {
        Self::try_from_row(row).expect("Decoding row failed")
    }

    fn try_from_row(row: &Row) -> Result<Self, Error> {
        let decoder = ModuleDecoderRegistry::default().with_fallback();

        let txid_bytes: Vec<u8> = row.try_get("txid")?;
        let txid =
            TransactionId::consensus_decode_vec(txid_bytes, &decoder).expect("Invalid data in DB");

        let session_index = row.try_get::<_, i32>("session_index")?;

        let item_index = row.try_get::<_, i32>("item_index")?;

        let data_bytes: Vec<u8> = row.try_get("data")?;
        let data =
            fedimint_core::transaction::Transaction::consensus_decode_vec(data_bytes, &decoder)
                .expect("Invalid data in DB");

        Ok(crate::federation::db::Transaction {
            txid,
            session_index,
            item_index,
            data,
        })
    }
}
