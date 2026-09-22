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

#[tokio::test]
async fn test_custom_mission_arbitrary_coordinates() {
    let app = create_app(PathBuf::from("."));

    let custom_payload = serde_json::json!({
        "system": "Earth-Moon",
        "origin_state": [0.8369, 0.0, 0.01, 0.0, 0.12, 0.0],
        "destination_state": [1.155, 0.0, 0.05, 0.0, -0.15, 0.0],
        "spacecraft_wet_mass_kg": 750.0,
        "max_thrust_n": 0.5,
        "isp_s": 2200.0,
        "flight_days": 16.0,
        "n_nodes": 20
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/transfer/optimize")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&custom_payload).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["converged"], true);
    assert_eq!(json["trajectory_nodes"].as_array().unwrap().len(), 20);
    assert!(json["total_fuel_consumed_kg"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_earth_centric_tli_staging_transfer() {
    let app = create_app(PathBuf::from("."));

    let payload = serde_json::json!({
        "system": "Earth-Moon",
        "origin_preset": "tli_staging",
        "destination_preset": "lunar_gateway_nrho",
        "leo_altitude_km": 400.0,
        "spacecraft_wet_mass_kg": 500.0,
        "max_thrust_n": 0.45,
        "isp_s": 2600.0,
        "flight_days": 16.0,
        "n_nodes": 25
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/transfer/optimize")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["converged"], true);

    // Impulsive TLI Delta-v from 400 km LEO should be around 3100-3130 m/s
    let tli_dv = json["tli_impulsive_delta_v_m_s"].as_f64().unwrap();
    assert!((tli_dv - 3120.0).abs() < 50.0);

    // Total mission Delta-v includes TLI + SCvx electric burn
    let total_dv = json["total_mission_delta_v_m_s"].as_f64().unwrap();
    let electric_dv = json["total_delta_v_m_s"].as_f64().unwrap();
    assert!((total_dv - (tli_dv + electric_dv)).abs() < 1e-6);
}

#[tokio::test]
async fn test_rpo_visualizer_route() {
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let app = create_app(workspace_root);

    let req = Request::builder()
        .uri("/rpo")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let html_str = String::from_utf8_lossy(&body);
    assert!(html_str.contains("Autonomous RPO & Clohessy-Wiltshire Visualizer"));
}

#[tokio::test]
async fn test_rpo_plan_endpoint() {
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let app = create_app(workspace_root);

    let payload = serde_json::json!({
        "target_orbit": "iss",
        "mode": "two_impulse",
        "initial_state": [-150.0, -800.0, 20.0, 0.0, 0.0, 0.0],
        "target_position": [0.0, -30.0, 0.0],
        "target_velocity": [0.0, 0.0, 0.0],
        "duration_s": 1800.0
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/rpo/plan")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["mode"], "Two-Impulse Targeted Rendezvous");
    assert!(json["total_delta_v_mps"].as_f64().unwrap() > 0.0);
    assert_eq!(json["burns"].as_array().unwrap().len(), 2);
    assert_eq!(json["trajectory_points"].as_array().unwrap().len(), 60);
}


