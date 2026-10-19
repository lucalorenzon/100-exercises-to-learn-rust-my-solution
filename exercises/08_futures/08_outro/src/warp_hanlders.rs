use std::sync::Arc;

use warp::reply::{Reply, Response};

use crate::{
    service::ForTicketMgr,
    ticket::{DraftTicket, PatchTicket},
};

pub async fn handler_ticket_add<T>(
    body: warp::hyper::body::Bytes,
    service: Arc<T>,
) -> Result<Response, warp::Rejection>
where
    T: ForTicketMgr + 'static,
{
    let Ok(draft_ticket) = serde_json::from_slice::<DraftTicket>(&body) else {
        return Err(warp::reject());
    };
    let ticket_id = service.add(&draft_ticket).await;
    Ok(warp::reply::json(&ticket_id).into_response())
}

pub async fn handler_ticket_get<T>(id: u64, service: Arc<T>) -> Result<Response, warp::Rejection>
where
    T: ForTicketMgr + 'static,
{
    let maybe_ticket = service.get(&id.into()).await;
    match maybe_ticket {
        Some(ticket) => Ok(warp::reply::json(&ticket).into_response()),
        None => Err(warp::reject()),
    }
}

pub async fn handler_ticket_patch<T>(
    id: u64,
    version: u64,
    body: warp::hyper::body::Bytes,
    service: Arc<T>,
) -> Result<Response, warp::Rejection>
where
    T: ForTicketMgr + 'static,
{
    let Ok(patch_ticket) = serde_json::from_slice::<PatchTicket>(&body) else {
        return Err(warp::reject());
    };
    let maybe_ticket = service
        .patch(&id.into(), &version.into(), patch_ticket)
        .await;
    match maybe_ticket {
        Ok(ticket) => Ok(warp::reply::json(&ticket).into_response()),
        Err(_) => Err(warp::reject()),
    }
}
