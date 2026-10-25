use std::{convert::Infallible, net::SocketAddr, sync::Arc};

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
    service::{ForAddTicket, ForGetTicket, ForPatchTicket, ForTicketMgr},
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
    T: ForAddTicket + 'static,
{
    warp::post()
        .and(path("tickets"))
        .and(end())
        .and(extract_request_context())
        .and(body::bytes())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_add)
}

fn get_ticket_route<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ForGetTicket + 'static,
{
    warp::get()
        .and(path("tickets"))
        .and(param::<u64>())
        .and(end())
        .and(extract_request_context())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_get)
}

fn patch_ticket_route<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ForPatchTicket + 'static,
{
    warp::patch()
        .and(path("tickets"))
        .and(param::<u64>())
        .and(end())
        .and(header::<u64>("x-version"))
        .and(extract_request_context())
        .and(body::bytes())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_patch)
}

pub struct RequestContext {
    pub(crate) forwarded_for: Option<String>,
    pub(crate) real_ip: Option<String>,
    pub(crate) remote_ip: Option<String>,
    pub(crate) user_agent: Option<String>,
}

fn extract_request_context() -> impl Filter<Extract = (RequestContext,), Error = Rejection> + Clone
{
    warp::header::optional::<String>("x-forwarded-for")
        .and(warp::header::optional::<String>("real-ip"))
        .and(warp::addr::remote())
        .and(warp::header::optional::<String>("user-agent"))
        .map(
            |forwarded_for, real_ip, remote_ip: Option<SocketAddr>, user_agent| RequestContext {
                forwarded_for,
                real_ip,
                remote_ip: remote_ip.map(|s: SocketAddr| s.to_string()),
                user_agent,
            },
        )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::{json, Value};
    use warp::http::StatusCode;

    use crate::{
        description::TicketDescription,
        service::{
            tests::{
                build_exhausted_service, build_service,
                build_service_that_return_ticket_0_with_specified_version,
            },
            ForAddTicket, TicketSystemError,
        },
        status::Status,
        ticket::{
            DraftTicket, PatchTicket, TicketId, TicketVersionMaxReachedError,
            TicketVersionMismatchError, VersionedTicket,
        },
        ticket_store::{TicketInsertError, TicketUpdateError},
        title::TicketTitle,
        warp_hanlders::HttpError,
        warp_routes::warp_routes,
    };

    #[tokio::test]
    async fn return_the_id_of_the_inserted_ticket() {
        let service = Arc::new(build_service());
        let draft_ticket = DraftTicket {
            title: "my_title".try_into().unwrap(),
            description: "my_desciption".try_into().unwrap(),
        };
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("POST")
            .path(&format!("/tickets"))
            .json(&draft_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let ticket_id =
            serde_json::from_slice::<TicketId>(&body).expect("doesn't return a valid json");
        assert_eq!(ticket_id, 0.into());
    }

    #[tokio::test]
    async fn return_insufficient_storage_if_reached_max_insertable_tickets() {
        let service = Arc::new(build_exhausted_service());
        let draft_ticket = DraftTicket {
            title: "my_title".try_into().unwrap(),
            description: "my_desciption".try_into().unwrap(),
        };
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("POST")
            .path(&format!("/tickets"))
            .json(&draft_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::INSUFFICIENT_STORAGE);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let err_json: Value = serde_json::from_slice(&body).expect("not a valid json");
        let expected_json = json!({
            "error": &HttpError::TicketIdSpaceExhausted(TicketSystemError::InsertError(TicketInsertError::StorageFull)).to_string()
        });
        assert_eq!(err_json, expected_json);
    }

    #[tokio::test]
    async fn return_the_specified_ticket() {
        let service = Arc::new(build_service());
        let expected_title: TicketTitle = "my_title".try_into().unwrap();
        let expected_decsription: TicketDescription = "my_desciption".try_into().unwrap();
        let ticket_id = service
            .add(&DraftTicket {
                title: expected_title.clone(),
                description: expected_decsription.clone(),
            })
            .await
            .unwrap();
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("GET")
            .path(&format!("/tickets/{ticket_id}"))
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let versioned_ticket =
            serde_json::from_slice::<VersionedTicket>(&body).expect("doesn't return a valid json");
        assert_eq!(versioned_ticket.id(), ticket_id);
        assert_eq!(versioned_ticket.title(), expected_title);
        assert_eq!(versioned_ticket.description(), expected_decsription);
        assert_eq!(versioned_ticket.version(), 0.into());
    }

    #[tokio::test]
    async fn should_return_not_found_if_requested_not_existent_ticket() {
        let service = Arc::new(build_service());
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("GET")
            .path("/tickets/100")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let err_json: Value = serde_json::from_slice(&body).expect("not a valid json");
        let expected_json = json!({
            "error": "TicketId 100 not found"
        });
        assert_eq!(err_json, expected_json);
    }

    #[tokio::test]
    async fn should_return_not_found_if_invalid_path_is_requested() {
        let invalid_paths = [
            "/",
            "/invalid",
            "/not/configured/path",
            "/..",
            "/../../../../../../../../../../etc/passwd",
        ];
        let service = Arc::new(build_service());
        for invalid_path in invalid_paths {
            let route = warp_routes(service.clone());
            let resp = warp::test::request()
                .method("GET")
                .path(invalid_path)
                .reply(&route)
                .await;
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            assert_eq!(resp.headers()["content-type"], "application/json");
            let body = resp.into_body();
            let err_json: Value = serde_json::from_slice(&body).expect("not a valid json");
            let expected_json = json!({
                "error": "Not Found"
            });
            assert_eq!(err_json, expected_json);
        }
    }

    #[tokio::test]
    async fn should_patch_the_version_of_the_specified_ticket() {
        let first_title: TicketTitle = "my_title".try_into().unwrap();
        let expected_title: TicketTitle = "modified_title".try_into().unwrap();
        let expected_description: TicketDescription = "my_desciption".try_into().unwrap();
        let patch_ticket = PatchTicket {
            title: Some(expected_title.clone()),
            ..PatchTicket::default()
        };

        let service = Arc::new(build_service());
        let draft_ticket = DraftTicket {
            title: first_title,
            description: expected_description.clone(),
        };
        let ticket_id = service.add(&draft_ticket).await.unwrap();
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("PATCH")
            .path(&format!("/tickets/{ticket_id}"))
            .header("x-version", 0)
            .json(&patch_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let versioned_ticket =
            serde_json::from_slice::<VersionedTicket>(&body).expect("doesn't return a valid json");
        assert_eq!(versioned_ticket.id(), 0.into());
        assert_eq!(versioned_ticket.title(), expected_title);
        assert_eq!(versioned_ticket.description(), expected_description);
        assert_eq!(versioned_ticket.status(), Status::ToDo);
        assert_eq!(versioned_ticket.version(), 1.into());
    }

    #[tokio::test]
    async fn should_return_conflic_patching_wrong_version_of_the_specified_ticket() {
        let first_title: TicketTitle = "my_title".try_into().unwrap();
        let expected_title: TicketTitle = "modified_title".try_into().unwrap();
        let expected_description: TicketDescription = "my_desciption".try_into().unwrap();
        let patch_ticket = PatchTicket {
            title: Some(expected_title.clone()),
            ..PatchTicket::default()
        };

        let service = Arc::new(build_service());
        let draft_ticket = DraftTicket {
            title: first_title,
            description: expected_description.clone(),
        };
        let ticket_id = service.add(&draft_ticket).await.unwrap();
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("PATCH")
            .path(&format!("/tickets/{ticket_id}"))
            .header("x-version", 1)
            .json(&patch_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let err_json: Value = serde_json::from_slice(&body).expect("not a valid json");
        let expected_json = json!({
            "error": &TicketSystemError::UpdateError(TicketUpdateError::Conflict(TicketVersionMismatchError)).to_string()
        });
        assert_eq!(err_json, expected_json);
    }

    #[tokio::test]
    async fn should_return_expectation_failed_patching_max_version_of_the_specified_ticket() {
        let expected_title: TicketTitle = "modified_title".try_into().unwrap();
        let patch_ticket = PatchTicket {
            title: Some(expected_title.clone()),
            ..PatchTicket::default()
        };

        let service = Arc::new(build_service_that_return_ticket_0_with_specified_version(
            u64::MAX,
        ));
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("PATCH")
            .path(&format!("/tickets/0"))
            .header("x-version", u64::MAX)
            .json(&patch_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::EXPECTATION_FAILED);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let err_json: Value = serde_json::from_slice(&body).expect("not a valid json");
        let expected_json = json!({
            "error": &TicketSystemError::UpdateError(TicketUpdateError::VersionExausted(TicketVersionMaxReachedError)).to_string()
        });
        assert_eq!(err_json, expected_json);
    }

    #[tokio::test]
    async fn should_return_not_found_patching_a_not_existing_ticket() {
        let expected_title: TicketTitle = "modified_title".try_into().unwrap();
        let patch_ticket = PatchTicket {
            title: Some(expected_title.clone()),
            ..PatchTicket::default()
        };

        let service = Arc::new(build_service());
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("PATCH")
            .path(&format!("/tickets/0"))
            .header("x-version", 0)
            .json(&patch_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let err_json: Value = serde_json::from_slice(&body).expect("not a valid json");
        let expected_json = json!({
            "error": &TicketSystemError::UpdateError(TicketUpdateError::NotFound(0.into())).to_string()
        });
        assert_eq!(err_json, expected_json);
    }
}
