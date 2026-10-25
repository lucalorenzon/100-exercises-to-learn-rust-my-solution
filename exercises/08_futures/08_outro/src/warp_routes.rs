use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use tracing::info_span;
use warp::{
    filters::{
        body,
        header::header,
        method::{patch, post},
        path::{end, param},
    },
    reject::Rejection,
    reply::Reply,
    Filter,
};

use crate::{
    service::{ForAddTicket, ForGetTicket, ForPatchTicket, ForTicketMgr},
    warp_handlers::{errors_handler, handler_ticket_add, handler_ticket_get, handler_ticket_patch},
};

const MAX_BODY_LENGTH: u64 = 2048;
const MAX_HEADER_LEN: usize = 255;

pub fn warp_routes<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone
where
    T: ForTicketMgr + 'static,
{
    request_context()
        .and(
            add_ticket_route(service.clone())
                .or(get_ticket_route(service.clone()))
                .or(patch_ticket_route(service.clone())),
        )
        .recover(errors_handler)
        .with(warp::trace(|info| {
            info_span!(
                "request",
                request_id = %uuid::Uuid::new_v4(),
                method =  %info.method(),
                path = %info.path(),
                remote_ip = tracing::field::Empty,
                x_forwarded_for = tracing::field::Empty,
                x_real_ip = tracing::field::Empty,
                user_agent = tracing::field::Empty,
                ticket_id = tracing::field::Empty
            )
        }))
}

fn add_ticket_route<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ForAddTicket + 'static,
{
    warp::path("tickets")
        .and(end())
        .and(post())
        .and(warp::body::content_length_limit(MAX_BODY_LENGTH))
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
    warp::path("tickets")
        .and(param::<u64>())
        .and(end())
        .and(warp::get())
        .and(warp::any().map(move || service.clone()))
        .and_then(handler_ticket_get)
}

fn patch_ticket_route<T>(
    service: Arc<T>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    T: ForPatchTicket + 'static,
{
    warp::path("tickets")
        .and(param::<u64>())
        .and(end())
        .and(patch())
        .and(header::<u64>("x-version"))
        .and(warp::body::content_length_limit(MAX_BODY_LENGTH))
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

fn request_context() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    extract_request_context()
        .map(|ctx: RequestContext| {
            let span = tracing::Span::current();
            span.record("remote_ip", ctx.remote_ip.as_deref().unwrap_or(""));
            span.record(
                "x_forwarded_for",
                ctx.forwarded_for.as_deref().unwrap_or(""),
            );
            span.record("x_real_ip", ctx.real_ip.as_deref().unwrap_or(""));
            span.record("user_agent", ctx.user_agent.as_deref().unwrap_or(""));
        })
        .untuple_one()
}

fn extract_request_context() -> impl Filter<Extract = (RequestContext,), Error = Rejection> + Clone
{
    warp::header::optional::<String>("x-forwarded-for")
        .and(warp::header::optional::<String>("x-real-ip"))
        .and(warp::addr::remote())
        .and(warp::header::optional::<String>("user-agent"))
        .map(
            |forwarded_for: Option<String>,
             real_ip: Option<String>,
             remote_ip: Option<SocketAddr>,
             user_agent: Option<String>| {
                RequestContext {
                    forwarded_for: forwarded_for.as_deref().map(sanitise_value),
                    real_ip: real_ip.as_deref().map(sanitise_value),
                    remote_ip: remote_ip.map(|s: SocketAddr| s.to_string()),
                    user_agent: user_agent.as_deref().map(sanitise_value),
                }
            },
        )
}

fn sanitise_value(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control())
        .take(MAX_HEADER_LEN)
        .collect()
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
            DraftTicket, PatchTicket, TicketVersionMaxReachedError, TicketVersionMismatchError,
            VersionedTicket,
        },
        ticket_store::{TicketInsertError, TicketUpdateError},
        title::TicketTitle,
        warp_handlers::{HttpError, TicketIdResponse},
        warp_routes::{warp_routes, MAX_BODY_LENGTH},
    };

    #[tokio::test]
    async fn return_the_id_of_the_inserted_ticket() {
        let service = Arc::new(build_service());
        let draft_ticket = DraftTicket {
            title: "my_title".try_into().unwrap(),
            description: "my_description".try_into().unwrap(),
        };
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("POST")
            .path("/tickets")
            .json(&draft_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let ticket_id_resp =
            serde_json::from_slice::<TicketIdResponse>(&body).expect("doesn't return a valid json");
        assert_eq!(ticket_id_resp.id, 0.into());
    }

    #[tokio::test]
    async fn return_insufficient_storage_if_reached_max_insertable_tickets() {
        let service = Arc::new(build_exhausted_service());
        let draft_ticket = DraftTicket {
            title: "my_title".try_into().unwrap(),
            description: "my_description".try_into().unwrap(),
        };
        let route = warp_routes(service);
        let resp = warp::test::request()
            .method("POST")
            .path("/tickets")
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
        let expected_description: TicketDescription = "my_description".try_into().unwrap();
        let ticket_id = service
            .add(DraftTicket {
                title: expected_title.clone(),
                description: expected_description.clone(),
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
        assert_eq!(versioned_ticket.description(), expected_description);
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
    async fn should_patch_the_version_of_the_specified_ticket() {
        let first_title: TicketTitle = "my_title".try_into().unwrap();
        let expected_title: TicketTitle = "modified_title".try_into().unwrap();
        let expected_description: TicketDescription = "my_description".try_into().unwrap();
        let patch_ticket = PatchTicket {
            title: Some(expected_title.clone()),
            ..PatchTicket::default()
        };

        let service = Arc::new(build_service());
        let draft_ticket = DraftTicket {
            title: first_title,
            description: expected_description.clone(),
        };
        let ticket_id = service.add(draft_ticket).await.unwrap();
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
    async fn should_return_conflict_patching_wrong_version_of_the_specified_ticket() {
        let first_title: TicketTitle = "my_title".try_into().unwrap();
        let expected_title: TicketTitle = "modified_title".try_into().unwrap();
        let expected_description: TicketDescription = "my_description".try_into().unwrap();
        let patch_ticket = PatchTicket {
            title: Some(expected_title.clone()),
            ..PatchTicket::default()
        };

        let service = Arc::new(build_service());
        let draft_ticket = DraftTicket {
            title: first_title,
            description: expected_description.clone(),
        };
        let ticket_id = service.add(draft_ticket).await.unwrap();
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
            .path("/tickets/0")
            .header("x-version", u64::MAX)
            .json(&patch_ticket)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::EXPECTATION_FAILED);
        assert_eq!(resp.headers()["content-type"], "application/json");
        let body = resp.into_body();
        let err_json: Value = serde_json::from_slice(&body).expect("not a valid json");
        let expected_json = json!({
            "error": &TicketSystemError::UpdateError(TicketUpdateError::VersionExhausted(TicketVersionMaxReachedError)).to_string()
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
            .path("/tickets/0")
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
    async fn should_return_not_allowed_method_if_wrong_method_is_used() {
        let service = Arc::new(build_service());
        let route = warp_routes(service.clone());
        let resp = warp::test::request()
            .method("DELETE")
            .path("/tickets/0")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn should_return_bad_request_if_payload_is_not_valid_json() {
        let service = Arc::new(build_service());
        let route = warp_routes(service.clone());
        let resp = warp::test::request()
            .method("POST")
            .path("/tickets")
            .body("{werwerwerwe")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn should_return_unprocessable_content_if_payload_cannot_be_deserialized() {
        let service = Arc::new(build_service());
        let route = warp_routes(service.clone());
        let body_with_wrong_description = json!({
            "title": "my-title",
            "description": "a".repeat(501)
        });
        let resp = warp::test::request()
            .method("POST")
            .path("/tickets")
            .body(body_with_wrong_description.to_string())
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn should_return_bad_request_if_version_header_is_missed() {
        let service = Arc::new(build_service());
        let route = warp_routes(service.clone());
        let resp = warp::test::request()
            .method("PATCH")
            .path("/tickets/0")
            .header("wrong-header", "invalid")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn should_return_bad_request_if_invalid_header_value_is_used() {
        let service = Arc::new(build_service());
        let route = warp_routes(service.clone());
        let resp = warp::test::request()
            .method("PATCH")
            .path("/tickets/0")
            .header("x-version", "invalid")
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn should_return_length_required_if_a_body_is_missed_in_patch() {
        let service = Arc::new(build_service());
        let route = warp_routes(service.clone());
        let resp = warp::test::request()
            .method("PATCH")
            .path("/tickets/0")
            .header("x-version", 0)
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::LENGTH_REQUIRED);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }

    #[tokio::test]
    async fn should_return_payload_too_large_if_a_body_is_too_big() {
        let service = Arc::new(build_service());
        let route = warp_routes(service.clone());
        let body_too_large = json!({
            "title": "my-title",
            "description": "a".repeat(MAX_BODY_LENGTH as usize + 1)
        });
        let resp = warp::test::request()
            .method("POST")
            .path("/tickets")
            .body(body_too_large.to_string())
            .reply(&route)
            .await;
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(resp.headers()["content-type"], "application/json");
    }
}
