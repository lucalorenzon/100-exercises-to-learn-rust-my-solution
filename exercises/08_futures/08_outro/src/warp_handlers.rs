use std::{convert::Infallible, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::error::Category;
use tracing::warn;
use warp::{
    http::StatusCode,
    reject::{self, Reject},
    reply::{Reply, Response},
};

use crate::{
    service::{ForAddTicket, ForGetTicket, ForPatchTicket, TicketSystemError},
    ticket::{DraftTicket, PatchTicket, TicketId},
    ticket_store::TicketUpdateError,
};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TicketIdResponse {
    pub id: TicketId,
}

impl From<TicketId> for TicketIdResponse {
    fn from(value: TicketId) -> Self {
        TicketIdResponse { id: value }
    }
}

pub async fn handler_ticket_add<T>(
    body: warp::hyper::body::Bytes,
    service: Arc<T>,
) -> Result<Response, warp::Rejection>
where
    T: ForAddTicket + 'static,
{
    let draft_ticket = serde_json::from_slice::<DraftTicket>(&body).map_err(invalid_payload)?;
    let ticket_id = service.add(draft_ticket).await.map_err(update_error)?;
    tracing::Span::current().record("ticket_id", ticket_id.to_string());
    Ok(warp::reply::json(&TicketIdResponse::from(ticket_id)).into_response())
}

fn invalid_payload(error: serde_json::Error) -> warp::Rejection {
    match error.classify() {
        Category::Data => {
            warn!(error = %error, "invalid content");
            warp::reject::custom(HttpError::InvalidContent("Json semantically incorrect"))
        }
        _ => {
            warn!(error = %error, "invalid payload");
            warp::reject::custom(HttpError::InvalidPayload("Json not correctly formatted"))
        }
    }
}

pub async fn handler_ticket_get<T>(id: u64, service: Arc<T>) -> Result<Response, warp::Rejection>
where
    T: ForGetTicket + 'static,
{
    tracing::Span::current().record("ticket_id", id.to_string());
    let maybe_ticket = service.get(&id.into()).await;
    match maybe_ticket {
        Some(ticket) => Ok(warp::reply::json(&ticket).into_response()),
        None => Err(warp::reject::custom(HttpError::TicketNotFound(id))),
    }
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
    tracing::Span::current().record("ticket_id", id.to_string());
    let patch_ticket = serde_json::from_slice::<PatchTicket>(&body).map_err(invalid_payload)?;
    let ticket = service
        .patch(&id.into(), &version.into(), patch_ticket)
        .await
        .map_err(update_error)?;

    Ok(warp::reply::json(&ticket).into_response())
}

fn update_error(value: TicketSystemError) -> warp::Rejection {
    match value {
        TicketSystemError::UpdateError(TicketUpdateError::Conflict(_)) => {
            warp::reject::custom(HttpError::TicketVersionConflict(value))
        }
        TicketSystemError::UpdateError(TicketUpdateError::VersionExhausted(_)) => {
            warp::reject::custom(HttpError::TicketVersionExhausted(value))
        }
        TicketSystemError::UpdateError(TicketUpdateError::NotFound(_)) => {
            warp::reject::custom(HttpError::TicketUpdateNotFound(value))
        }
        TicketSystemError::InsertError(_) => {
            warp::reject::custom(HttpError::TicketIdSpaceExhausted(value))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub enum HttpError {
    InvalidContent(&'static str),
    InvalidPayload(&'static str),
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
            HttpError::InvalidContent(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()),
            HttpError::InvalidPayload(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            HttpError::TicketIdSpaceExhausted(e) => {
                (StatusCode::INSUFFICIENT_STORAGE, e.to_string())
            }
            HttpError::TicketVersionExhausted(e) => (StatusCode::EXPECTATION_FAILED, e.to_string()),
            HttpError::TicketVersionConflict(e) => (StatusCode::CONFLICT, e.to_string()),
        }
    } else if let Some(e) = err.find::<reject::PayloadTooLarge>() {
        (StatusCode::PAYLOAD_TOO_LARGE, e.to_string())
    } else if let Some(e) = err.find::<reject::LengthRequired>() {
        (StatusCode::LENGTH_REQUIRED, e.to_string())
    } else if let Some(e) = err.find::<reject::MissingHeader>() {
        (StatusCode::BAD_REQUEST, e.to_string())
    } else if let Some(e) = err.find::<reject::InvalidHeader>() {
        (StatusCode::BAD_REQUEST, e.to_string())
    } else if let Some(e) = err.find::<reject::MethodNotAllowed>() {
        (StatusCode::METHOD_NOT_ALLOWED, e.to_string())
    } else if err.is_not_found() {
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    } else {
        tracing::error!("unhandled rejection: {err:?}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error".to_string(),
        )
    };
    let json = warp::reply::json(&serde_json::json!({ "error": &msg }));
    Ok(warp::reply::with_status(json, code))
}
