use std::net::SocketAddr;

use axum::{Router, Server};
use oj_judger::judge;
use tracing::debug;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    fmt::init();

    let n = 1_000_000_0;
    let a = vec![1; n];
    debug!("{}", a[n - 1] * 2 * a[n - 2] * a[n / 2]);

    let app = Router::new();
    let app = app.merge(Router::new().nest("/judge", judge::get_router()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3002));
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
