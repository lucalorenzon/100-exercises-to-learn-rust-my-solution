use std::net::SocketAddr;

use warp::{filters::BoxedFilter, reply::Response};

pub struct WarpHttpServer {
    routes: BoxedFilter<(Response,)>,
}

impl WarpHttpServer {
    pub fn new(routes: BoxedFilter<(Response,)>) -> Self {
        Self { routes }
    }

    pub async fn serve(&self, socket_address: SocketAddr) {
        warp::serve(self.routes.clone()).run(socket_address).await
    }
}
