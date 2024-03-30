use axum::extract::Path;
use axum::Json;
use fedimint_core::api::InviteCode;
use fedimint_core::config::FederationId;

pub async fn fetch_federation_id(
    Path(invite): Path<InviteCode>,
) -> crate::error::Result<Json<FederationId>> {
    Ok(invite.federation_id().into())
}
