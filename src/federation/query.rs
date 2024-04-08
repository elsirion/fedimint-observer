use anyhow::Context;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_auth::AuthBearer;
use hex::ToHex;
use serde::Serialize;
use sqlx::postgres::any::AnyTypeInfoKind;
use sqlx::{query, Any, Column, Database, Row};
use tracing::debug;
use tracing::log::info;

use crate::federation::observer::FederationObserver;
use crate::AppState;

pub async fn run_query(
    AuthBearer(auth): AuthBearer,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> crate::error::Result<Response> {
    let observer = state.federation_observer;

    observer.check_auth(&auth)?;

    let query = body
        .get("query")
        .context("No query provided")?
        .as_str()
        .context("Query parameter wasn't a string")?;
    debug!("Running query: {query}");
    let result = observer.run_qery(query).await?;
    debug!("Query result: {result:?}");

    const CSV_CONTENT_TYPE: &[u8] = b"text/csv";
    let content_type = headers
        .get(axum::http::header::ACCEPT)
        .map(|accept| accept.as_bytes());
    let response = if content_type == Some(CSV_CONTENT_TYPE) {
        let mut csv_writer = csv::Writer::from_writer(Vec::<u8>::new());

        csv_writer
            .write_record(result.cols)
            .context("Writing headers failed")?;
        for row in result.rows {
            csv_writer.serialize(row).context("Writing row failed")?;
        }

        let csv_bytes = csv_writer.into_inner().context("Writing CSV failed")?;
        Response::builder()
            .header(axum::http::header::CONTENT_TYPE, CSV_CONTENT_TYPE)
            .body(Body::from(csv_bytes))
            .context("error creating response")?
    } else {
        Json::from(result).into_response()
    };

    Ok(response)
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    cols: Vec<String>,
    rows: Vec<Vec<serde_json::Value>>,
}

impl FederationObserver {
    /// Runs a SQL query against the database and outputs thew result as a JSON
    /// encodable `QueryResult`.
    pub async fn run_qery(&self, sql: &str) -> anyhow::Result<QueryResult> {
        let result: Vec<<Any as Database>::Row> = query(sql)
            .fetch_all(self.connection().await?.as_mut())
            .await?;

        let Some(first_row) = result.first() else {
            return Ok(QueryResult {
                cols: vec![],
                rows: vec![],
            });
        };

        let cols = first_row
            .columns()
            .iter()
            .map(|col| col.name().to_owned())
            .collect();

        info!("cols: {cols:?}");

        let rows = result
            .into_iter()
            .map(|row| {
                row.columns()
                    .iter()
                    .map(|col| {
                        let col_type = col.type_info();

                        match col_type.kind() {
                            AnyTypeInfoKind::Null => row
                                .try_get::<bool, _>(col.ordinal())
                                .ok()
                                .map(Into::<serde_json::Value>::into)
                                .or_else(|| {
                                    row.try_get::<String, _>(col.ordinal()).ok().map(Into::into)
                                })
                                .or_else(|| {
                                    row.try_get::<i64, _>(col.ordinal()).ok().map(Into::into)
                                })
                                .or_else(|| {
                                    row.try_get::<Vec<u8>, _>(col.ordinal())
                                        .ok()
                                        .map(|bytes| bytes.encode_hex::<String>().into())
                                })
                                .into(),
                            AnyTypeInfoKind::Bool => {
                                row.try_get::<bool, _>(col.ordinal()).ok().into()
                            }
                            AnyTypeInfoKind::SmallInt
                            | AnyTypeInfoKind::Integer
                            | AnyTypeInfoKind::BigInt => {
                                row.try_get::<i64, _>(col.ordinal()).ok().into()
                            }
                            AnyTypeInfoKind::Real | AnyTypeInfoKind::Double => {
                                row.try_get::<f64, _>(col.ordinal()).ok().into()
                            }
                            AnyTypeInfoKind::Text => {
                                row.try_get::<String, _>(col.ordinal()).ok().into()
                            }
                            AnyTypeInfoKind::Blob => row
                                .try_get::<Vec<u8>, _>(col.ordinal())
                                .ok()
                                .map(|bytes| bytes.encode_hex::<String>())
                                .into(),
                        }
                    })
                    .collect()
            })
            .collect();

        Ok(QueryResult { cols, rows })
    }
}
