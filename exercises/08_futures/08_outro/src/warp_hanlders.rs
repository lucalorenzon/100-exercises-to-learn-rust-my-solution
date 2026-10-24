use std::{convert::Infallible, sync::Arc};

use serde_json::error::Category;
use tracing::{error, warn, Instrument};
use warp::{
    http::StatusCode,
    reject::Reject,
    reply::{Reply, Response},
};

use crate::{
    service::{ForAddTicket, ForGetTicket, ForPatchTicket, TicketSystemError},
    ticket::{DraftTicket, PatchTicket},
    ticket_store::TicketUpdateError,
};

pub async fn handler_ticket_add<T>(
    body: warp::hyper::body::Bytes,
    service: Arc<T>,
) -> Result<Response, warp::Rejection>
where
    T: ForAddTicket + 'static,
{
    let request_id = uuid::Uuid::new_v4();
    let span = tracing::info_span!(
        "request-add-ticket",
        request_id = %request_id
    );
    async {
        let draft_ticket = serde_json::from_slice::<DraftTicket>(&body).map_err(invalid_payload)?;
        let ticket_id = service
            .add(&draft_ticket)
            .await
            .map_err(|e| update_error(e))?;
        tracing::Span::current().record("ticket_id", ticket_id.to_string());
        Ok(warp::reply::json(&ticket_id).into_response())
    }
    .instrument(span)
    .await
}

fn invalid_payload(error: serde_json::Error) -> warp::Rejection {
    tracing::error!(error = %error, "invalid JSON payload");
    match error.classify() {
        Category::Data => warp::reject::custom(HttpError::InvalidContent(error)),
        _ => warp::reject::custom(HttpError::InvalidPayload(error)),
    }
}

pub async fn handler_ticket_get<T>(id: u64, service: Arc<T>) -> Result<Response, warp::Rejection>
where
    T: ForGetTicket + 'static,
{
    let request_id = uuid::Uuid::new_v4();
    let span = tracing::info_span!(
        "request-get-ticket",
        request_id = %request_id,
        ticket_id = %id
    );
    async {
        let maybe_ticket = service.get(&id.into()).await;
        match maybe_ticket {
            Some(ticket) => Ok(warp::reply::json(&ticket).into_response()),
            None => Err(warp::reject::custom(HttpError::TicketNotFound(id))),
        }
    }
    .instrument(span)
    .await
}

pub async fn handler_ticket_patch<T>(
    id: u64,
    version: u64,
    body: warp::hyper::body::Bytes,
    service: Arc<T>,
) -> Result<Response, warp::Rejection>
where
    T: ForPatchTicket + 'static,
{
    let request_id = uuid::Uuid::new_v4();
    let span = tracing::info_span!(
        "request-patch-ticket",
        request_id = %request_id,
        ticket_id = %id,
        version = %version
    );
    async {
        let patch_ticket = serde_json::from_slice::<PatchTicket>(&body).map_err(invalid_payload)?;
        let ticket = service
            .patch(&id.into(), &version.into(), patch_ticket)
            .await
            .map_err(|e| update_error(e))?;

        Ok(warp::reply::json(&ticket).into_response())
    }
    .instrument(span)
    .await
}

fn update_error(value: TicketSystemError) -> warp::Rejection {
    match value {
        TicketSystemError::UpdateError(TicketUpdateError::Conflict(_)) => {
            warn!(error=%value, "Ticket Update Version conflict");
            warp::reject::custom(HttpError::TicketVersionConflict(value))
        }
        TicketSystemError::UpdateError(TicketUpdateError::VersionExausted(_)) => {
            error!(error=%value, "Ticket Update Version exhausted");
            warp::reject::custom(HttpError::TicketVersionExhausted(value))
        }
        TicketSystemError::UpdateError(TicketUpdateError::NotFound(_)) => {
            error!(error=%value, "Ticket to update not found");
            warp::reject::custom(HttpError::TicketUpdateNotFound(value))
        }
        TicketSystemError::InsertError(_) => {
            error!(error=%value, "Ticket version exhausted");
            warp::reject::custom(HttpError::TicketIdSpaceExhausted(value))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub enum HttpError {
    InvalidPayload(serde_json::Error),
    InvalidContent(serde_json::Error),
    TicketIdSpaceExhausted(TicketSystemError),
    TicketNotFound(u64),
    TicketUpdateNotFound(TicketSystemError),
    TicketVersionExhausted(TicketSystemError),
    TicketVersionConflict(TicketSystemError),
}

impl Reject for HttpError {}

pub async fn errors_handler(err: warp::Rejection) -> Result<impl Reply, Infallible> {
    let (code, msg) = if let Some(err) = err.find::<HttpError>() {
        match err {
            HttpError::TicketNotFound(id) => {
                (StatusCode::NOT_FOUND, format!("TicketId {id} not found"))
            }
            HttpError::TicketUpdateNotFound(e) => (StatusCode::NOT_FOUND, e.to_string()),
            HttpError::InvalidPayload(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            HttpError::InvalidContent(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
            HttpError::TicketIdSpaceExhausted(e) => {
                (StatusCode::INSUFFICIENT_STORAGE, e.to_string())
            }
            HttpError::TicketVersionExhausted(e) => (StatusCode::EXPECTATION_FAILED, e.to_string()),
            HttpError::TicketVersionConflict(e) => (StatusCode::CONFLICT, e.to_string()),
        }
    } else {
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    };
    let json = warp::reply::json(&serde_json::json!({ "error": msg }));
    Ok(warp::reply::with_status(json, code))
}
