# Complete API & CLI Interface Specification

**System**: Space Debris Breakup & Astrodynamics Mission Planning Platform (`sbm_visualizer`)  
**Version**: 0.2.0  
**Status**: Production & Verified  
**Architecture Document**: [ADR-002: Native Client-Server Architecture](file:///Users/tomohiko/work/sbm_visualizer/docs/adr/ADR-002-deep-space-scvx-native-client-server-architecture.md)  
**Base Server URL**: `http://127.0.0.1:8080` (Local Native Daemon)  

---

## Table of Contents
1. [Architecture Overview & Topology](#1-architecture-overview--topology)
2. [Native CLI Interface Specification (`sbm_cli`)](#2-native-cli-interface-specification-sbm_cli)
   - [CLI Global Syntax & Flags](#21-cli-global-syntax--flags)
   - [Subcommand: `sbm_cli rpo`](#22-subcommand-sbm_cli-rpo)
   - [Subcommand: `sbm_cli transfer`](#23-subcommand-sbm_cli-transfer)
   - [Subcommand: `sbm_cli breakup`](#24-subcommand-sbm_cli-breakup)
   - [Subcommand: `sbm_cli help`](#25-subcommand-sbm_cli-help)
3. [RESTful HTTP JSON API Specification (`sbm_server`)](#3-restful-http-json-api-specification-sbm_server)
   - [Health & Capabilities (`GET /api/v1/health`)](#31-system-health--capabilities-get-apiv1health)
   - [Gravitational Systems & Lagrange Points (`GET /api/v1/system/:name`)](#32-system-parameters--lagrange-points-get-apiv1systemname)
   - [Lyapunov Orbit Differential Correction (`POST /api/v1/orbit/correct/lyapunov`)](#33-planar-lyapunov-orbit-correction-post-apiv1orbitcorrectlyapunov)
   - [3D Halo Orbit Differential Correction (`POST /api/v1/orbit/correct/halo`)](#34-3d-halo-orbit-correction-post-apiv1orbitcorrecthalo)
   - [Invariant Manifold Generation (`POST /api/v1/orbit/manifold`)](#35-invariant-manifold-generation-post-apiv1orbitmanifold)
   - [CR3BP Benchmark Transfer (`GET /api/v1/transfer/benchmark`)](#36-cr3bp-low-energy-transfer-benchmark-get-apiv1transferbenchmark)
   - [Successive Convexification Optimizer (`POST /api/v1/transfer/optimize`)](#37-scvx-trajectory-optimizer-post-apiv1transferoptimize)
   - [Flight Ephemeris Export (`POST /api/v1/export/oem`)](#38-flight-ephemeris-export-post-apiv1exportoem)
   - [RPO Maneuver Planning (`POST /api/v1/rpo/plan`)](#39-rpo-target-maneuver-planner-post-apiv1rpoplan)
4. [Static Web Visualizer Endpoints](#4-static-web-visualizer-endpoints)
5. [Data Units & Coordinate Frames Reference](#5-data-units--coordinate-frames-reference)
6. [Complete End-to-End Automation Pipeline Example](#6-complete-end-to-end-automation-pipeline-example)

---

## 1. Architecture Overview & Topology

The platform provides a unified dual-interface architecture where all flight dynamics, trajectory optimization, and relative motion capabilities can be accessed headlessly from terminal shells or automated pipelines:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        User & Flight Operations                        │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
         ┌──────────────────────────┴──────────────────────────┐
         ▼                                                     ▼
┌───────────────────────────────┐             ┌────────────────────────────────┐
│      Native Shell CLI         │             │    Local REST API (Daemon)     │
│       `sbm_simple_engine`     │             │     `http://127.0.0.1:8080`    │
│    (Compiled via `sbm_cli`)   │             │   (Axum 0.7 + Tokio Daemon)    │
└───────────────┬───────────────┘             └────────────────┬───────────────┘
                │                                              │
                │        ┌────────────────────────────┐        │
                ├───────►│  sbm_core Astrodynamics   │◄───────┤
                │        │  - NASA EVOLVE 4.0 Breakup │        │
                │        │  - CR3BP 9D Symplectic EOM │        │
                │        │  - Successive Convex (SCvx)│        │
                │        │  - Clohessy-Wiltshire (CW) │        │
                │        └────────────────────────────┘        │
                ▼                                              ▼
┌───────────────────────────────┐             ┌────────────────────────────────┐
│  Stdout Log & JSON Artifacts  │             │   Web Cockpits & Three.js 3D   │
│   (`fragments_output.json`)   │             │  `/cr3bp_deep_space_visualizer`│
│                               │             │  `/rpo_visualizer.html`        │
└───────────────────────────────┘             └────────────────────────────────┘
```

* **Crate `crates/sbm_core`**: Zero-dependency pure-Rust astrodynamics and trajectory optimization engine.
* **Crate `crates/sbm_server`**: Native Axum 0.7 RESTful daemon with CORS enabled (`*`) running locally.
* **Crate `engine_cli` (`sbm_cli`)**: High-performance headless command-line tool.

---

## 2. Native CLI Interface Specification (`sbm_cli`)

### 2.1. CLI Global Syntax & Flags

```bash
cargo run -p sbm_cli -- <COMMAND> [OPTIONS]
# Or, if installed via cargo install --path engine_cli:
sbm_simple_engine <COMMAND> [OPTIONS]
```

#### Environment Variables
* `RUST_LOG=info|debug|trace|warn|error`: Controls logging verbosity. Default is `info`.
* `RUST_BACKTRACE=1`: Prints stack traces upon unexpected failures.

---

### 2.2. Subcommand: `sbm_cli rpo`

Computes relative proximity maneuvers in the target-centered Local-Vertical Local-Horizontal (LVLH / Hill) coordinate frame.

#### Syntax & Options
```bash
cargo run -p sbm_cli -- rpo [OPTIONS]
```

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `--mode` | string | `two-impulse` | RPO profile: `two-impulse`, `nmc`, `vbar`, or `rbar` |
| `--target` | string | `iss` | Target reference orbit preset: `iss` (400 km), `sso` (700 km), or `geo` (35,786 km) |
| `--x` | float | `-100.0` | Initial radial offset $x_0$ in meters (+X = away from Earth, -X = Earth side) |
| `--y` | float | `-500.0` | Initial along-track offset $y_0$ in meters (+Y = ahead, -Y = behind) |
| `--z` | float | `0.0` | Initial cross-track offset $z_0$ in meters (+Z = normal to orbit plane) |
| `--duration` | float | `1800.0` | Transfer time of flight $\Delta t$ in seconds (e.g. 1800s = 30 min) |
| `--radial` | float | `100.0` | NMC radial semi-axis amplitude $A_x$ in meters |

#### Examples

##### Example 1: Two-Impulse Targeted Intercept
```bash
cargo run -p sbm_cli -- rpo --mode two-impulse --target iss --x -150 --y -800 --duration 1800
```
*Output*:
```log
INFO Target orbit configured target="iss" altitude_km=400.0 mean_motion_rad_s=0.001131 period_min=92.56
INFO Two-Impulse Targeted Transfer Planned departure_pos=[-150.0, -800.0, 0.0] target_pos=[0.0, -30.0, 0.0] duration_min=30.0
INFO Maneuver 1 (Departure) time_s=0.0 delta_v_mps=[-0.237, 0.386, 0.0] magnitude_mps=0.453 m/s desc="Departure Injection Burn"
INFO Maneuver 2 (Arrival Braking) time_s=1800.0 delta_v_mps=[-0.341, -0.047, 0.0] magnitude_mps=0.345 m/s desc="Arrival Braking & Insertion Burn"
INFO Total Maneuver Budget total_delta_v_mps=0.798 m/s
```

##### Example 2: Natural Motion Circumnavigation (NMC 360° Passive Inspection)
```bash
cargo run -p sbm_cli -- rpo --mode nmc --target iss --radial 120
```
*Output*:
```log
INFO Target orbit configured target="iss" altitude_km=400.0 mean_motion_rad_s=0.001131 period_min=92.56
INFO Passive Natural Motion Circumnavigation planned radial_amplitude_m=120.0 along_track_amplitude_m=240.0 cross_track_amplitude_m=50.0 period_min=92.56
INFO Insertion Maneuver (Drift-Free 2:1 Ellipse) burn_time_s=0.0 delta_v_mps=[0.0, 0.0, 0.0] magnitude_mps=0.0
INFO RPO Plan Complete - Zero continuous propellant drift total_delta_v_mps=0.0
```

---

### 2.3. Subcommand: `sbm_cli transfer`

Computes three-body invariant manifold heteroclinic transfers (AAS 20-459 benchmark).

#### Syntax
```bash
cargo run -p sbm_cli -- transfer
```
*Output*:
```log
INFO System parameters loaded system="Earth-Moon" mass_ratio_mu=0.01215 characteristic_length_km=384400000.0
INFO Low-energy heteroclinic transfer evaluated (AAS 20-459)
     departure_orbit="Earth-Moon L1 Southern Lyapunov"
     target_orbit="Earth-Moon L2 Southern Lyapunov"
     dv1_mps=0.00019 dv2_mps=23.20 dv3_mps=0.009 total_delta_v_mps=23.209 m/s
     pos_match_error_m=2.68e-5 vel_match_error_mps=5.68e-14
```

---

### 2.4. Subcommand: `sbm_cli breakup`

Executes the NASA EVOLVE 4.0 Standard Breakup Model collision simulation.

#### Syntax
```bash
cargo run -p sbm_cli -- breakup
# (or simply `cargo run -p sbm_cli` with no arguments)
```
*Output*:
```log
INFO Configured target target_mass_kg=1000.0 target_type=Spacecraft
INFO Configured projectile projectile_mass_kg=100.0 projectile_type=Spacecraft
INFO Collision parameters impact_velocity_km_s=10.0 specific_energy_kj_per_kg=4545.45
INFO Breakup regime evaluated outcome="CATASTROPHIC (Total breakup)" destroyed_mass_kg=1100.0
INFO Debris cloud population generated yield_1cm=15243 yield_10cm_ssn=842 sampled_fragments=1500
INFO Successfully exported fragments output_file="fragments_output.json" fragments_count=1500
```
Generates `fragments_output.json` for ingestion into the 3D WebGL breakup visualizer.

---

### 2.5. Subcommand: `sbm_cli help`
Displays usage manual, options, and commands.

---

## 3. RESTful HTTP JSON API Specification (`sbm_server`)

### 3.1. System Health & Capabilities (`GET /api/v1/health`)

#### Endpoint Summary
* **Method**: `GET`
* **Path**: `/api/v1/health`
* **Description**: Returns daemon health status, version, and supported engine features.

#### Example Request
```bash
curl -s http://127.0.0.1:8080/api/v1/health | jq .
```

#### Example Response (`200 OK`)
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

---

### 3.2. System Parameters & Lagrange Points (`GET /api/v1/system/:name`)

#### Endpoint Summary
* **Method**: `GET`
* **Path**: `/api/v1/system/:name`
* **Path Parameters**:
  * `:name`: System identifier: `earth_moon`, `sun_earth`, or `sun_jupiter`.
* **Description**: Returns characteristic scale units and exact coordinates of all 5 Lagrange points ($L_1$ to $L_5$).

#### Example Request
```bash
curl -s http://127.0.0.1:8080/api/v1/system/earth_moon | jq .
```

#### Example Response (`200 OK`)
```json
{
  "name": "earth_moon",
  "mu": 0.012150584077904827,
  "l_star_km": 384400.0,
  "t_star_days": 4.3425,
  "v_star_ms": 1024.55,
  "lagrange_points": [
    {
      "point": "L1",
      "x": 0.836915,
      "y": 0.0,
      "z": 0.0,
      "jacobi_constant": 3.18834,
      "distance_to_primary_km": 321710.1,
      "distance_to_secondary_km": 62689.9
    },
    {
      "point": "L2",
      "x": 1.155682,
      "y": 0.0,
      "z": 0.0,
      "jacobi_constant": 3.17216,
      "distance_to_primary_km": 444244.2,
      "distance_to_secondary_km": 59844.2
    }
  ]
}
```

---

### 3.3. Planar Lyapunov Orbit Correction (`POST /api/v1/orbit/correct/lyapunov`)

#### Endpoint Summary
* **Method**: `POST`
* **Path**: `/api/v1/orbit/correct/lyapunov`
* **Request Headers**: `Content-Type: application/json`

#### Request JSON Schema
```json
{
  "system": "earth_moon",
  "x0": 0.8369,
  "vy0_guess": 0.12,
  "max_iter": 20,
  "tol": 1e-8
}
```

#### Example Request
```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/orbit/correct/lyapunov \
  -H "Content-Type: application/json" \
  -d '{"system":"earth_moon","x0":0.8369,"vy0_guess":0.12,"max_iter":20,"tol":1e-8}' | jq .
```

#### Response JSON Schema (`200 OK`)
```json
{
  "success": true,
  "initial_state": [0.8369, 0.0, 0.0, 0.0, 0.1258, 0.0],
  "period_nondim": 2.742,
  "period_days": 11.91,
  "jacobi_constant": 3.1630,
  "stability_index": 12.45,
  "lambda_unstable": 22.3,
  "lambda_stable": 0.0448,
  "eigenvector_unstable": [0.35, 0.12, 0.0, 0.0, 0.82, 0.0],
  "eigenvector_stable": [0.35, -0.12, 0.0, 0.0, -0.82, 0.0],
  "iterations": 4,
  "residual": 1.2e-9,
  "trajectory_points": [[0.0, 0.8369, 0.0, 0.0], ...]
}
```

---

### 3.4. 3D Halo Orbit Correction (`POST /api/v1/orbit/correct/halo`)

#### Endpoint Summary
* **Method**: `POST`
* **Path**: `/api/v1/orbit/correct/halo`

#### Request JSON Schema
```json
{
  "system": "earth_moon",
  "z0": 0.045,
  "x0_guess": 0.8234,
  "vy0_guess": 0.13,
  "max_iter": 20,
  "tol": 1e-8
}
```

#### Example Request
```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/orbit/correct/halo \
  -H "Content-Type: application/json" \
  -d '{"system":"earth_moon","z0":0.045,"x0_guess":0.8234,"vy0_guess":0.13}' | jq .
```

---

### 3.5. Invariant Manifold Generation (`POST /api/v1/orbit/manifold`)

#### Endpoint Summary
* **Method**: `POST`
* **Path**: `/api/v1/orbit/manifold`

#### Request JSON Schema
```json
{
  "system": "earth_moon",
  "initial_state": [0.8369, 0.0, 0.0, 0.0, 0.1258, 0.0],
  "period_nondim": 2.742,
  "eigenvector_unstable": [0.35, 0.12, 0.0, 0.0, 0.82, 0.0],
  "eigenvector_stable": [0.35, -0.12, 0.0, 0.0, -0.82, 0.0],
  "manifold_type": "unstable",
  "branch": "positive",
  "orbit_phase": 0.25,
  "epsilon_dist": 1e-5,
  "t_span": 2.5
}
```

---

### 3.6. CR3BP Low-Energy Transfer Benchmark (`GET /api/v1/transfer/benchmark`)

#### Endpoint Summary
* **Method**: `GET`
* **Path**: `/api/v1/transfer/benchmark`
* **Description**: Returns the AAS 20-459 Section 5 verified $L_1 \to L_2$ three-maneuver heteroclinic transfer itinerary ($\Delta v = 23.21\text{ m/s}$).

#### Example Request
```bash
curl -s http://127.0.0.1:8080/api/v1/transfer/benchmark | jq .
```

#### Example Response (`200 OK`)
```json
{
  "reference_paper": "AAS 20-459 Section 5 (Short, Haapala, Bosanac 2020)",
  "total_delta_v_ms": 23.209,
  "dv1_ms": 0.00019,
  "dv2_ms": 23.20,
  "dv3_ms": 0.009,
  "position_match_error_m": 0.000027,
  "velocity_match_error_ms": 0.000000,
  "departure_orbit": "Earth-Moon L1 Southern Lyapunov",
  "arrival_orbit": "Earth-Moon L2 Southern Lyapunov"
}
```

---

### 3.7. SCvx Trajectory Optimizer (`POST /api/v1/transfer/optimize`)

#### Endpoint Summary
* **Method**: `POST`
* **Path**: `/api/v1/transfer/optimize`
* **Description**: Runs Successive Convexification (SCvx) low-thrust trajectory optimization connecting an Earth-centric orbit (LEO, GTO, or GEO) or Lagrange point to any target destination with automatic TLI kick calculation.

#### Request JSON Schema
```json
{
  "system": "earth_moon",
  "origin_preset": "tli_staging",
  "origin_state": null,
  "destination_preset": "earth_moon_l1_halo",
  "destination_state": null,
  "leo_altitude_km": 400.0,
  "spacecraft_wet_mass_kg": 500.0,
  "max_thrust_n": 0.5,
  "isp_s": 3000.0,
  "flight_days": 14.0,
  "n_nodes": 30
}
```

#### Supported Presets
* **`origin_preset`**: `tli_staging`, `trans_lunar_injection`, `earth_gto_apogee`, `earth_geo`, `earth_moon_l1_halo`, `earth_moon_l1_lyapunov`, `low_lunar_orbit`, `lunar_gateway_nrho`.
* **`destination_preset`**: `earth_moon_l1_halo`, `earth_moon_l1_lyapunov`, `earth_moon_l2_halo`, `earth_moon_l2_lyapunov`, `low_lunar_orbit`, `lunar_gateway_nrho`, `earth_moon_l4`, `earth_moon_l5`, `sun_earth_l2_halo`.

#### Example Request
```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/transfer/optimize \
  -H "Content-Type: application/json" \
  -d '{
    "system": "earth_moon",
    "origin_preset": "tli_staging",
    "leo_altitude_km": 400.0,
    "destination_preset": "earth_moon_l1_halo",
    "spacecraft_wet_mass_kg": 500.0,
    "max_thrust_n": 0.5,
    "isp_s": 3000.0,
    "flight_days": 14.0,
    "n_nodes": 30
  }' | jq '{converged: .converged, tli_dv: .tli_impulsive_delta_v_m_s, electric_dv: .total_delta_v_m_s, total_mission_dv: .total_mission_delta_v_m_s}'
```

#### Response JSON Schema (`200 OK`)
```json
{
  "success": true,
  "converged": true,
  "iterations": 6,
  "total_flight_days": 14.0,
  "total_delta_v_m_s": 48.21,
  "tli_impulsive_delta_v_m_s": 3123.45,
  "total_mission_delta_v_m_s": 3171.66,
  "total_fuel_consumed_kg": 0.82,
  "final_mass_kg": 499.18,
  "max_thrust_used_mn": 500.0,
  "burn_schedule": [
    {
      "segment_index": 1,
      "start_day": 0.0,
      "end_day": 1.4,
      "duration_hours": 33.6,
      "average_thrust_mn": 480.0,
      "delta_v_m_s": 24.1,
      "fuel_consumed_kg": 0.41
    }
  ],
  "trajectory_nodes": [
    {
      "time_days": 0.0,
      "tau": 0.0,
      "position_km": [-6778.0, 0.0, 0.0],
      "velocity_km_s": [0.0, 10.85, 0.0],
      "thrust_accel_m_s2": [0.0008, 0.0006, 0.0],
      "thrust_mn": 500.0,
      "cumulative_delta_v_m_s": 0.0
    }
  ]
}
```

---

### 3.8. Flight Ephemeris Export (`POST /api/v1/export/oem`)

#### Endpoint Summary
* **Method**: `POST`
* **Path**: `/api/v1/export/oem`
* **Description**: Formats optimization trajectory nodes into an official **CCSDS OEM v2.0 ASCII ephemeris** file and an **RFC 4180 CSV** tabular dataset.

#### Request JSON Schema
```json
{
  "object_name": "GATEWAY-LOGISTICS-01",
  "object_id": "2026-092A",
  "center_name": "EARTH-MOON BARYCENTER",
  "ref_frame": "EM_ROTATING",
  "time_system": "UTC",
  "nodes": [
    {
      "time_days": 0.0,
      "tau": 0.0,
      "position_km": [-6778.0, 0.0, 0.0],
      "velocity_km_s": [0.0, 10.85, 0.0],
      "thrust_accel_m_s2": [0.001, 0.0, 0.0],
      "thrust_mn": 500.0,
      "cumulative_delta_v_m_s": 0.0
    }
  ]
}
```

#### Example Request
```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/export/oem \
  -H "Content-Type: application/json" \
  -d '{"object_name":"SAT-01","nodes":[{"time_days":0.0,"tau":0.0,"position_km":[1000,0,0],"velocity_km_s":[0,1,0],"thrust_accel_m_s2":[0,0,0],"thrust_mn":0,"cumulative_delta_v_m_s":0}]}' | jq -r '.oem_content'
```

---

### 3.9. RPO Target Maneuver Planner (`POST /api/v1/rpo/plan`)

#### Endpoint Summary
* **Method**: `POST`
* **Path**: `/api/v1/rpo/plan`
* **Description**: Computes relative proximity maneuvers in Hill/LVLH coordinates.

#### Request Parameters
| Parameter | Type | Required | Default | Description |
| :--- | :--- | :--- | :--- | :--- |
| `target_orbit` | string | No | `"iss"` | Target orbit preset: `"iss"`, `"sso"`, or `"geo"` |
| `target_altitude_km`| float | No | `400.0` | Custom circular orbit altitude in km |
| `mode` | string | No | `"two_impulse"` | `"two_impulse"`, `"nmc"`, `"vbar"`, or `"rbar"` |
| `initial_state` | [f64; 6]| No | `[-100, -500, 0, 0, 0, 0]` | Initial relative state $[x_0, y_0, z_0, \dot{x}_0, \dot{y}_0, \dot{z}_0]$ in $m$ and $m/s$ |
| `target_position` | [f64; 3]| No | `[0, -30, 0]` | Desired relative position $[x_f, y_f, z_f]$ in meters |
| `target_velocity` | [f64; 3]| No | `[0, 0, 0]` | Desired arrival relative velocity in $m/s$ |
| `duration_s` | float | No | `1800.0` | Transfer duration in seconds |
| `nmc_radial_amplitude_m` | float | No | `100.0` | NMC radial semi-axis ($A_x$) in meters |
| `nmc_cross_track_amplitude_m` | float | No | `50.0` | NMC cross-track amplitude ($A_z$) in meters |
| `glideslope_start_m` | float | No | `-400.0` | Start distance for V-bar or R-bar glideslope |
| `glideslope_end_m` | float | No | `-15.0` | Final hold distance |
| `num_hops` | usize | No | `3` | Number of discrete hops for V-bar approach |

#### Example Request
```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/rpo/plan \
  -H "Content-Type: application/json" \
  -d '{
    "target_orbit": "iss",
    "mode": "two_impulse",
    "initial_state": [-100.0, -500.0, 0.0, 0.0, 0.0, 0.0],
    "target_position": [0.0, -20.0, 0.0],
    "duration_s": 1800.0
  }' | jq .
```

#### Response JSON Schema (`200 OK`)
```json
{
  "success": true,
  "mode": "Two-Impulse Targeted Rendezvous",
  "target_orbit": "400.0 km circular",
  "mean_motion_rad_s": 0.00113136665,
  "orbital_period_s": 5553.62,
  "total_delta_v_mps": 0.4996,
  "burns": [
    {
      "time_s": 0.0,
      "delta_v_mps": [-0.1401, 0.2519, 0.0],
      "magnitude_mps": 0.2882,
      "description": "Departure Injection Burn"
    },
    {
      "time_s": 1800.0,
      "delta_v_mps": [-0.2098, -0.0257, 0.0],
      "magnitude_mps": 0.2114,
      "description": "Arrival Braking & Insertion Burn"
    }
  ],
  "trajectory_points": [
    [0.0, -100.0, -500.0, 0.0],
    [30.5, -104.18, -492.17, 0.0],
    [1800.0, 0.0, -20.0, 0.0]
  ]
}
```

---

## 4. Static Web Visualizer Endpoints

| Route | Content | Description |
| :--- | :--- | :--- |
| `GET /` or `GET /cr3bp_deep_space_visualizer.html` | HTML / WebGL | 3D Deep Space Mission Planning Cockpit & Invariant Manifold Visualizer |
| `GET /rpo` or `GET /rpo_visualizer.html` | HTML / WebGL | 3D Interactive Target-Centered LVLH Proximity Operations Visualizer |
| `GET /index.html` | HTML / WebGL | NASA EVOLVE 4.0 Breakup Cloud & Gabbard Diagram Visualizer |

---

## 5. Data Units & Coordinate Frames Reference

### Physical Quantities & Standard Units
| Domain | Metric | Canonical Unit | Alternate Unit |
| :--- | :--- | :--- | :--- |
| **Position / Distance** | Deep Space CR3BP | Kilometers ($km$) | Non-dimensional $l^*$ |
| **Position / Distance** | RPO Proximity Operations | Meters ($m$) | — |
| **Velocity** | Deep Space CR3BP | Kilometers/second ($km/s$) | Non-dimensional $v^*$ |
| **Velocity** | RPO Relative Velocity | Meters/second ($m/s$) | — |
| **Delta-V ($\Delta v$)** | Maneuver Budget | Meters/second ($m/s$) | — |
| **Time** | Deep Space Flight Time | Days ($days$) | Non-dimensional $\tau$ |
| **Time** | RPO Scenario Time | Seconds ($s$) | Minutes ($min$) |
| **Thrust Force** | Propulsion Rating | Newtons ($N$) | milliNewtons ($mN$) |
| **Specific Impulse ($I_{sp}$)** | Engine Efficiency | Seconds ($s$) | — |
| **Mass** | Spacecraft / Debris | Kilograms ($kg$) | — |

### Coordinate Frames
1. **CR3BP Barycentric Rotating Frame**:
   * Origin: Center of mass of the two primary bodies (e.g. Earth and Moon).
   * $+X$: Along primary-to-secondary vector.
   * $+Z$: Along orbital angular momentum of the primaries.
   * $+Y$: Completes the right-handed orthogonal triad.
2. **Hill / LVLH Rotating Relative Frame**:
   * Origin: Center of mass of the target satellite.
   * $+X$ (Radial / R-bar): Along the position vector pointing outward from Earth center.
   * $+Y$ (In-Track / V-bar): Along the target's orbital velocity vector.
   * $+Z$ (Cross-Track / H-bar): Along the target's angular momentum vector $\mathbf{h} = \mathbf{r} \times \mathbf{v}$.

---

## 6. Complete End-to-End Automation Pipeline Example

A complete bash automation script chaining optimization, ephemeris export, and RPO docking:

```bash
#!/usr/bin/env bash
set -euo pipefail

SERVER="http://127.0.0.1:8080"

echo "=== 1. Health Verification ==="
curl -sf "$SERVER/api/v1/health" | jq -r '.engine'

echo "=== 2. Optimizing LEO-to-Gateway SCvx Transit ==="
OPT_PLAN=$(curl -sf -X POST "$SERVER/api/v1/transfer/optimize" \
  -H "Content-Type: application/json" \
  -d '{
    "system": "earth_moon",
    "origin_preset": "tli_staging",
    "leo_altitude_km": 400.0,
    "destination_preset": "earth_moon_l1_halo",
    "spacecraft_wet_mass_kg": 750.0,
    "max_thrust_n": 0.6,
    "isp_s": 3100.0,
    "flight_days": 14.0,
    "n_nodes": 30
  }')

echo "Optimization Converged: $(echo "$OPT_PLAN" | jq .converged)"
echo "Total Mission Delta-V: $(echo "$OPT_PLAN" | jq .total_mission_delta_v_m_s) m/s"

echo "=== 3. Exporting Flight OEM Ephemeris ==="
curl -sf -X POST "$SERVER/api/v1/export/oem" \
  -H "Content-Type: application/json" \
  -d "{
    \"object_name\": \"MISSION-PATHFINDER-01\",
    \"nodes\": $(echo "$OPT_PLAN" | jq .trajectory_nodes)
  }" | jq -r '.oem_content' > mission_trajectory.oem

echo "Generated: mission_trajectory.oem ($(wc -l < mission_trajectory.oem) lines)"

echo "=== 4. Planning Terminal RPO Docking Intercept ==="
RPO_PLAN=$(curl -sf -X POST "$SERVER/api/v1/rpo/plan" \
  -H "Content-Type: application/json" \
  -d '{
    "target_orbit": "iss",
    "mode": "two_impulse",
    "initial_state": [-150.0, -800.0, 20.0, 0.0, 0.0, 0.0],
    "target_position": [0.0, -20.0, 0.0],
    "duration_s": 1800.0
  }')

echo "RPO Docking Delta-V Budget: $(echo "$RPO_PLAN" | jq .total_delta_v_mps) m/s"
echo "=== Mission Pipeline Successfully Automated! ==="
```
