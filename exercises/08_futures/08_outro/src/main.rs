use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use outro_08::{
    service::TicketSystemService, ticket_store::SimpleTicketStore, warp_routes::warp_routes,
    warp_server::WarpHttpServer,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_observability();
    info!(
        host = %"0.0.0.0",
        port = %3000,
        "starting server"
    );
    let storage_service = SimpleTicketStore::new();
    let service = TicketSystemService::new(storage_service);
    let server = WarpHttpServer::new(warp_routes(Arc::new(service)));
    server
        .serve(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000))
        .await;
}

fn init_observability() {
    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}
