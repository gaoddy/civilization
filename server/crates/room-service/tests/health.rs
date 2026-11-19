//! 健康检查路由：健康探针与 WebSocket 共存，未配置数据库时均可用。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use room_service::SystemClock;
use tower::ServiceExt;

async fn app() -> axum::Router {
    let (_addr, router, _state) = room_service::bind_state("127.0.0.1:0", Arc::new(SystemClock))
        .await
        .expect("路由应能装配");
    router
}

#[tokio::test]
async fn healthz_is_ok() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn readyz_without_database_is_ok() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ws_route_is_registered() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/ws").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_path_is_not_found() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
