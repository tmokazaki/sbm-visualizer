use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use sbm_server::create_app;
use std::path::PathBuf;
use tower::ServiceExt;

#[tokio::test]
async fn test_nbody_presets_endpoint() {
    let app = create_app(PathBuf::from("."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/nbody/presets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let presets = json.as_array().expect("presets must be an array");
    assert!(presets.len() >= 6, "Must provide at least 6 presets");

    let ids: Vec<&str> = presets
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"solar_system"));
    assert!(ids.contains(&"laplace_resonance"));
    assert!(ids.contains(&"figure_eight"));
}

#[tokio::test]
async fn test_nbody_simulate_endpoint() {
    let app = create_app(PathBuf::from("."));

    let req_body = serde_json::json!({
        "preset_id": "figure_eight",
        "total_duration_s": 50.0,
        "output_step_s": 10.0,
        "integrator": "yoshida4th"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/nbody/simulate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["bodies"].as_array().unwrap().len(), 3);
    let snapshots = json["snapshots"].as_array().unwrap();
    assert_eq!(snapshots.len(), 6); // t = 0, 10, 20, 30, 40, 50
    assert!(json["relative_energy_error"].as_f64().unwrap() < 1e-5);
}
