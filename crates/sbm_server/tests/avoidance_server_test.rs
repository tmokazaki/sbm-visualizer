//! Integration tests for space debris avoidance REST API endpoints.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use sbm_server::create_app;
use std::path::PathBuf;
use tower::ServiceExt;

#[tokio::test]
async fn test_avoidance_simulate_single_ppo() {
    let app = create_app(PathBuf::from("."));

    let req_body = serde_json::json!({
        "seed": 12345,
        "policy": "ppo",
        "max_steps": 100
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/avoidance/simulate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["comparative"], false);
    let episodes = json["episodes"].as_array().expect("episodes must be an array");
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0]["policy_name"], "PPO");
    assert!(!episodes[0]["snapshots"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_avoidance_simulate_comparative_overlay() {
    let app = create_app(PathBuf::from("."));

    let req_body = serde_json::json!({
        "seed": 12345,
        "policy": "comparative",
        "max_steps": 50
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/avoidance/simulate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["comparative"], true);
    let episodes = json["episodes"].as_array().expect("episodes must be an array");
    assert_eq!(episodes.len(), 4, "Comparative mode must return all 4 policies");
    assert_eq!(episodes[0]["policy_name"], "PPO");
    assert_eq!(episodes[1]["policy_name"], "Impulsive");
    assert_eq!(episodes[2]["policy_name"], "Rule-based");
    assert_eq!(episodes[3]["policy_name"], "No-action");
}

#[tokio::test]
async fn test_avoidance_training_data_endpoint() {
    let app = create_app(PathBuf::from("."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/avoidance/training-data")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let curves = json["curves"].as_array().expect("curves must be an array");
    assert_eq!(curves.len(), 101, "Curriculum curves must have 101 data points");
    assert_eq!(curves[0]["step"], 0);
    assert_eq!(curves[100]["step"], 1_000_000);

    let checkpoints = json["checkpoint_episodes"].as_array().expect("checkpoint_episodes array");
    assert_eq!(checkpoints.len(), 4, "Must provide 4 milestone checkpoint replays");
}

#[tokio::test]
async fn test_avoidance_real_scenario_endpoint() {
    let app = create_app(PathBuf::from("."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/avoidance/real-scenario?satellite=ISS&debris=COSMOS_2251")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["satellite_name"].as_str().unwrap().contains("ISS"));
    assert!(json["debris_catalog_name"].as_str().unwrap().contains("COSMOS"));
    assert!(json["unmaneuvered_miss_distance_m"].as_f64().unwrap() <= 300.0);
    assert!(json["achieved_clearance_m"].as_f64().unwrap() > 300.0);
}

#[tokio::test]
async fn test_avoidance_html_route() {
    let app = create_app(PathBuf::from("."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/avoidance")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("SBM AVOIDANCE COCKPIT"));
    assert!(html.contains("Multi-Policy Overlay"));
}

