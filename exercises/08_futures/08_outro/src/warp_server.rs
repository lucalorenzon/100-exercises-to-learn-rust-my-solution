use std::{convert::Infallible, net::SocketAddr};

use warp::{reply::Reply, Filter};

pub struct WarpHttpServer<F> {
    routes: F,
}

impl<F> WarpHttpServer<F>
where
    F: Filter<Error = Infallible> + Clone + Send + Sync + 'static,
    F::Extract: Reply,
{
    pub fn new(routes: F) -> Self {
        Self { routes }
    }

    pub async fn serve(&self, socket_address: SocketAddr) {
        warp::serve(self.routes.clone()).run(socket_address).await
    }
}
