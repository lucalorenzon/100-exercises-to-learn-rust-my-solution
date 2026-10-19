use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use outro_08::{
    service::TicketSystemService, ticket_store::SimpleTicketStore, warp_routes::warp_routes,
    warp_server::WarpHttpServer,
};

#[tokio::main]
async fn main() {
    let storage_service = SimpleTicketStore::new();
    let service = TicketSystemService::new(storage_service);
    let server = WarpHttpServer::new(warp_routes(Arc::new(service)));
    server
        .serve(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000))
        .await;
}
