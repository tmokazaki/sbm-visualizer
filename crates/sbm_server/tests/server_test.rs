use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use sbm_server::create_app;
use std::path::PathBuf;
use tower::ServiceExt;

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_app(PathBuf::from("."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], "0.2.0");
    assert!(json["capabilities"].as_array().unwrap().len() >= 5);
}

#[tokio::test]
async fn test_system_endpoint() {
    let app = create_app(PathBuf::from("."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/earth_moon")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["name"], "earth_moon");
    assert!((json["mu"].as_f64().unwrap() - 0.01215).abs() < 1e-4);
    assert_eq!(json["lagrange_points"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn test_transfer_benchmark_endpoint() {
    let app = create_app(PathBuf::from("."));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/transfer/benchmark")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["name"], "Earth-Moon L1 to L2 Low-Energy Multi-Body Transfer (AAS 20-459)");
    let total_dv = json["total_dv_ms"].as_f64().unwrap();
    assert!((total_dv - 23.2).abs() < 2.0);
}

#[tokio::test]
async fn test_optimize_transfer_and_export_oem() {
    let app = create_app(PathBuf::from("."));

    let opt_payload = serde_json::json!({
        "system": "Earth-Moon",
        "origin_preset": "earth_moon_l1_lyapunov",
        "destination_preset": "earth_moon_l2_halo",
        "spacecraft_wet_mass_kg": 450.0,
        "max_thrust_n": 0.35,
        "isp_s": 2800.0,
        "flight_days": 14.0,
        "n_nodes": 20
    });

    let opt_req = Request::builder()
        .method("POST")
        .uri("/api/v1/transfer/optimize")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&opt_payload).unwrap()))
        .unwrap();

    let opt_resp = app.clone().oneshot(opt_req).await.unwrap();
    assert_eq!(opt_resp.status(), StatusCode::OK);

    let opt_body = opt_resp.into_body().collect().await.unwrap().to_bytes();
    let opt_json: serde_json::Value = serde_json::from_slice(&opt_body).unwrap();

    assert_eq!(opt_json["success"], true);
    assert_eq!(opt_json["converged"], true);
    assert!(opt_json["iterations"].as_u64().unwrap() <= 10);
    assert!(opt_json["total_delta_v_m_s"].as_f64().unwrap() > 0.0);

    let nodes = opt_json["trajectory_nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 20);

    // Now test OEM Export endpoint using the optimized nodes
    let export_payload = serde_json::json!({
        "object_name": "TEST_SCVX_ORBITER",
        "object_id": "2026-042A",
        "nodes": nodes
    });

    let export_req = Request::builder()
        .method("POST")
        .uri("/api/v1/export/oem")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&export_payload).unwrap()))
        .unwrap();

    let export_resp = app.oneshot(export_req).await.unwrap();
    assert_eq!(export_resp.status(), StatusCode::OK);

    let export_body = export_resp.into_body().collect().await.unwrap().to_bytes();
    let export_json: serde_json::Value = serde_json::from_slice(&export_body).unwrap();

    assert_eq!(export_json["success"], true);
    let oem_str = export_json["oem_content"].as_str().unwrap();
    let csv_str = export_json["csv_content"].as_str().unwrap();

    assert!(oem_str.contains("CCSDS_OEM_VERS = 2.0"));
    assert!(oem_str.contains("OBJECT_NAME          = TEST_SCVX_ORBITER"));
    assert!(csv_str.contains("time_days,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,thrust_mn,cumulative_dv_m_s"));
}
