use std::{convert::Infallible, sync::Arc};

use warp::{
    filters::{
        body,
        header::header,
        path::{end, param, path},
    },
    reject::Rejection,
    reply::Reply,
    Filter,
};

use crate::{
    service::ForTicketMgr,
    warp_hanlders::{errors_handler, handler_ticket_add, handler_ticket_get, handler_ticket_patch},
};

pub fn warp_routes<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone
where
    T: ForTicketMgr + 'static,
{
    add_ticket_route(service.clone())
        .or(get_ticket_route(service.clone()))
        .or(patch_ticket_route(service.clone()))
        .recover(errors_handler)
}

fn add_ticket_route<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ForTicketMgr + 'static,
{
    warp::post()
        .and(path("tickets"))
        .and(end())
        .and(body::bytes())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_add)
}

fn get_ticket_route<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ForTicketMgr + 'static,
{
    warp::get()
        .and(path("tickets"))
        .and(param::<u64>())
        .and(end())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_get)
}

fn patch_ticket_route<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ForTicketMgr + 'static,
{
    warp::patch()
        .and(path("tickets"))
        .and(param::<u64>())
        .and(end())
        .and(header::<u64>("x-version"))
        .and(body::bytes())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_patch)
}
