use std::{convert::Infallible, sync::Arc};

use serde_json::error::Category;
use tracing::{error, info, Instrument};
use warp::{
    http::StatusCode,
    reject::Reject,
    reply::{Reply, Response},
};

use crate::{
    service::{ForAddTicket, ForGetTicket, ForPatchTicket, TicketSystemError},
    ticket::{DraftTicket, PatchTicket},
    ticket_store::TicketUpdateError,
    warp_routes::RequestContext,
};

pub async fn handler_ticket_add<T>(
    context: RequestContext,
    body: warp::hyper::body::Bytes,
    service: Arc<T>,
) -> Result<Response, warp::Rejection>
where
    T: ForAddTicket + 'static,
{
    let request_id = uuid::Uuid::new_v4();
    let span = tracing::info_span!(
        "request-add-ticket",
        x_forwarded_for = context.forwarded_for.as_deref().unwrap_or(""),
        x_real_ip = context.real_ip.as_deref().unwrap_or(""),
        remote_ip = context.remote_ip.as_deref().unwrap_or(""),
        user_agent = context.user_agent.as_deref().unwrap_or(""),
        request_id = %request_id,
        ticket_id = tracing::field::Empty
    );
    async {
        let draft_ticket = serde_json::from_slice::<DraftTicket>(&body).map_err(|e| {
            error!("Invalid Payload: {e}");
            invalid_payload(e)
        })?;
        let ticket_id = service.add(&draft_ticket).await.map_err(|e| {
            error!("Error adding ticket: {e}");
            update_error(e)
        })?;
        tracing::Span::current().record("ticket_id", ticket_id.to_string());
        Ok(warp::reply::json(&ticket_id).into_response())
    }
    .instrument(span)
    .await
}

fn invalid_payload(error: serde_json::Error) -> warp::Rejection {
    match error.classify() {
        Category::Data => warp::reject::custom(HttpError::InvalidContent(error)),
        _ => warp::reject::custom(HttpError::InvalidPayload(error)),
    }
}

pub async fn handler_ticket_get<T>(
    id: u64,
    context: RequestContext,
    service: Arc<T>,
) -> Result<Response, warp::Rejection>
where
    T: ForGetTicket + 'static,
{
    let request_id = uuid::Uuid::new_v4();
    let span = tracing::info_span!(
        "request-get-ticket",
        request_id = %request_id,
        x_forwarded_for = context.forwarded_for.as_deref().unwrap_or(""),
        x_real_ip = context.real_ip.as_deref().unwrap_or(""),
        remote_ip = context.remote_ip.as_deref().unwrap_or(""),
        user_agent = context.user_agent.as_deref().unwrap_or(""),
        ticket_id = %id
    );
    async {
        let maybe_ticket = service.get(&id.into()).await;
        match maybe_ticket {
            Some(ticket) => {
                info!("Found Ticket: {id}");
                Ok(warp::reply::json(&ticket).into_response())
            }
            None => {
                info!("Ticket {id} not found");
                Err(warp::reject::custom(HttpError::TicketNotFound(id)))
            }
        }
    }
    .instrument(span)
    .await
}

pub async fn handler_ticket_patch<T>(
    id: u64,
    version: u64,
    context: RequestContext,
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
        x_forwarded_for = context.forwarded_for.as_deref().unwrap_or(""),
        x_real_ip = context.real_ip.as_deref().unwrap_or(""),
        remote_ip = context.remote_ip.as_deref().unwrap_or(""),
        user_agent = context.user_agent.as_deref().unwrap_or(""),
        ticket_id = %id,
        version = %version
    );
    async {
        let patch_ticket = serde_json::from_slice::<PatchTicket>(&body).map_err(|e| {
            error!("Invalid Payload {e}");
            invalid_payload(e)
        })?;
        let ticket = service
            .patch(&id.into(), &version.into(), patch_ticket)
            .await
            .map_err(|e| {
                error!("Error patching Ticket {id}: {e}");
                update_error(e)
            })?;

        Ok(warp::reply::json(&ticket).into_response())
    }
    .instrument(span)
    .await
}

fn update_error(value: TicketSystemError) -> warp::Rejection {
    match value {
        TicketSystemError::UpdateError(TicketUpdateError::Conflict(_)) => {
            warp::reject::custom(HttpError::TicketVersionConflict(value))
        }
        TicketSystemError::UpdateError(TicketUpdateError::VersionExausted(_)) => {
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
    let json = warp::reply::json(&serde_json::json!({ "error": &msg }));
    error!(msg);
    Ok(warp::reply::with_status(json, code))
}
