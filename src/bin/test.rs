use axum::{Router, Server};
use oj_judger::judge;
use std::net::SocketAddr;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    fmt::init();

    let app = Router::new();
    let app = app.merge(Router::new().nest("/judge", judge::get_router()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3002));
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
