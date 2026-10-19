use std::sync::Arc;

use warp::{
    filters::{
        body,
        header::header,
        path::{end, param, path},
        BoxedFilter,
    },
    reply::Response,
    Filter,
};

use crate::{
    service::ForTicketMgr,
    warp_hanlders::{handler_ticket_add, handler_ticket_get, handler_ticket_patch},
};

pub fn warp_routes<T>(service: Arc<T>) -> BoxedFilter<(Response,)>
where
    T: ForTicketMgr + 'static,
{
    add_ticket_route(service.clone())
        .or(get_ticket_route(service.clone()))
        .unify()
        .or(patch_ticket_route(service.clone()))
        .unify()
        .boxed()
}

fn add_ticket_route<T>(service: Arc<T>) -> BoxedFilter<(Response,)>
where
    T: ForTicketMgr + 'static,
{
    warp::post()
        .and(path("tickets"))
        .and(end())
        .and(body::bytes())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_add)
        .boxed()
}

fn get_ticket_route<T>(service: Arc<T>) -> BoxedFilter<(Response,)>
where
    T: ForTicketMgr + 'static,
{
    warp::get()
        .and(path("tickets"))
        .and(param::<u64>())
        .and(end())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_get)
        .boxed()
}

fn patch_ticket_route<T>(service: Arc<T>) -> BoxedFilter<(Response,)>
where
    T: ForTicketMgr + 'static,
{
    warp::patch()
        .and(path("tickets"))
        .and(path("patch"))
        .and(param::<u64>())
        .and(end())
        .and(header::<u64>("x-version"))
        .and(body::bytes())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_patch)
        .boxed()
}
