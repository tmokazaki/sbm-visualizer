# `sbm_server`

High-performance, local-first native astrodynamics daemon and mission planning REST API server built with [Axum](https://github.com/tokio-rs/axum) and [Tokio](https://tokio.rs/).

Provides high-speed endpoints for cislunar flight dynamics, Successive Convexification (SCvx) trajectory optimization, differential orbit correction, invariant manifold generation, and standard flight product exports (CCSDS OEM v2.0 & RFC 4180 CSV).

---

## Getting Started

### Launch Server
```bash
# Run the daemon locally (default: http://127.0.0.1:8080):
cargo run -p sbm_server

# Custom port and log filter:
PORT=9000 RUST_LOG=sbm_server=debug cargo run -p sbm_server
```

Once running, navigate to:
* **Interactive CR3BP Mission Cockpit**: [`http://127.0.0.1:8080/`](http://127.0.0.1:8080/)
* **NASA SBM Breakup Visualizer**: [`http://127.0.0.1:8080/index.html`](http://127.0.0.1:8080/index.html)
* **Daemon Health Check**: [`http://127.0.0.1:8080/api/v1/health`](http://127.0.0.1:8080/api/v1/health)

---

## REST API Endpoints

### 1. Health & Capabilities
```http
GET /api/v1/health
```
**Response (200 OK)**:
```json
{
  "status": "ok",
  "version": "0.2.0",
  "engine": "CR3BP High-Precision Astrodynamics Engine (AAS 20-459)",
  "capabilities": [
    "Euler Quintic Lagrange Root Solving (L1-L5)",
    "Symplectic State Transition Matrix (STM) Variational Integration",
    "Single-Shooting Planar Lyapunov Orbit Differential Correction",
    "Two-Variable 3D Halo Orbit Differential Correction",
    "Monodromy Matrix Stability & Floquet Eigendecomposition",
    "Invariant Manifold Tube Generation (W^u, W^s)",
    "AAS 20-459 Low-Energy Multi-Body Transfer Engine"
  ]
}
```

### 2. System Canonical Parameters & Libration Points
```http
GET /api/v1/system/earth_moon
GET /api/v1/system/sun_earth
```

### 3. Successive Convexification (SCvx) Transfer Optimization
```http
POST /api/v1/transfer/optimize
Content-Type: application/json

{
  "system": "Earth-Moon",
  "origin_preset": "earth_moon_l1_lyapunov",
  "destination_preset": "earth_moon_l2_halo",
  "spacecraft_wet_mass_kg": 450.0,
  "max_thrust_n": 0.35,
  "isp_s": 2800.0,
  "flight_days": 14.0,
  "n_nodes": 30
}
```
**Response (200 OK)**:
Returns convergence status, number of successions, total flight days, $\Delta v$, fuel consumed, discrete operational burn schedules, and 3D trajectory nodes.

### 4. Navigation Ephemeris Export (CCSDS OEM v2.0 & CSV)
```http
POST /api/v1/export/oem
Content-Type: application/json

{
  "object_name": "LUNAR_GATEWAY_EXPLORER",
  "object_id": "2026-088A",
  "nodes": [ ... ]
}
```
**Response (200 OK)**:
```json
{
  "success": true,
  "format": "CCSDS OEM v2.0 & RFC 4180 CSV",
  "oem_content": "CCSDS_OEM_VERS = 2.0\nCREATION_DATE = ...",
  "csv_content": "time_days,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s,thrust_mn,cumulative_dv_m_s\n..."
}
```

---

## Integration Testing

`sbm_server` features an automated, in-memory integration test suite executing via `tower::ServiceExt::oneshot`:

```bash
cargo test -p sbm_server --test server_test
```
