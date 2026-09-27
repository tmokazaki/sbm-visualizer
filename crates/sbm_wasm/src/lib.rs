//! High-performance WebAssembly bindings for astronomical N-body dynamics,
//! Keplerian swarms, spatial gravitational fields, and satellite telemetry.

use wasm_bindgen::prelude::*;
use js_sys::Float64Array;
use core::f64::consts::PI;

use sbm_core::nbody::{
    compute_conservation_metrics, compute_pairwise_forces, compute_shadow_cone_geometry,
    compute_spatial_field_point, compute_vis_viva_speed, create_preset, evaluate_eclipse_state,
    step_system, CelestialBody, EclipseState, IntegratorType, NBodySystem, PresetId, G_STANDARD,
};

/// High-performance N-Body gravitational dynamics simulation engine.
#[wasm_bindgen]
pub struct WasmNBodyEngine {
    system: NBodySystem,
    preset_name: String,
}

#[wasm_bindgen]
impl WasmNBodyEngine {
    /// Initializes an N-Body system from a canonical astronomical preset.
    #[wasm_bindgen(constructor)]
    pub fn new(preset_id: &str) -> Result<WasmNBodyEngine, JsValue> {
        let pid = match preset_id {
            "solar_system" => PresetId::SolarSystemJpl,
            "inner_solar_system_jupiter" => PresetId::InnerSolarSystemJupiter,
            "laplace_resonance" => PresetId::LaplaceResonance,
            "figure_eight" => PresetId::FigureEight,
            "mercury_gr" => PresetId::RelativisticMercury,
            "pythagorean" => PresetId::Pythagorean3Body,
            "trojan_asteroids" => PresetId::SunJupiterTrojans,
            "trappist1" => PresetId::Trappist1Chain,
            _ => PresetId::InnerSolarSystemJupiter,
        };

        let system = create_preset(pid);
        Ok(WasmNBodyEngine {
            system,
            preset_name: preset_id.to_string(),
        })
    }

    /// Advances the N-body system by time `dt` seconds across `sub_steps` integration increments.
    pub fn step(&mut self, dt: f64, sub_steps: usize, integrator: &str, enable_gr: bool) {
        let int_type = match integrator {
            "yoshida6" => IntegratorType::Yoshida6th,
            "hermite4" => IntegratorType::Hermite4th,
            "leapfrog" => IntegratorType::Leapfrog,
            "dormand_prince" => IntegratorType::DormandPrince853,
            _ => IntegratorType::Yoshida4th,
        };

        self.system.integrator = int_type;
        self.system.enable_general_relativity = enable_gr;

        let sub_dt = dt / (sub_steps.max(1) as f64);
        for _ in 0..sub_steps.max(1) {
            step_system(&mut self.system, sub_dt);
        }
    }

    /// Returns bodies serialized to a JavaScript array of objects.
    pub fn get_bodies(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.system.bodies)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Returns flat array of body positions: [x0, y0, z0, x1, y1, z1, ...].
    pub fn get_positions_flat(&self) -> Float64Array {
        let mut flat = Vec::with_capacity(self.system.bodies.len() * 3);
        for b in &self.system.bodies {
            flat.extend_from_slice(&b.position_m);
        }
        Float64Array::from(flat.as_slice())
    }

    /// Returns flat array of body velocities: [vx0, vy0, vz0, vx1, vy1, vz1, ...].
    pub fn get_velocities_flat(&self) -> Float64Array {
        let mut flat = Vec::with_capacity(self.system.bodies.len() * 3);
        for b in &self.system.bodies {
            flat.extend_from_slice(&b.velocity_mps);
        }
        Float64Array::from(flat.as_slice())
    }

    /// Evaluates total mechanical energy, momentum, and barycenter conservation metrics.
    pub fn get_conservation_metrics(&self) -> Result<JsValue, JsValue> {
        let metrics = compute_conservation_metrics(&self.system);
        serde_wasm_bindgen::to_value(&metrics)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Evaluates pairwise gravitational forces on the body at `focus_index`.
    pub fn get_pairwise_forces(&self, focus_index: usize) -> Result<JsValue, JsValue> {
        let forces = compute_pairwise_forces(&self.system, focus_index);
        serde_wasm_bindgen::to_value(&forces)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Returns the preset identifier string.
    pub fn get_preset_name(&self) -> String {
        self.preset_name.clone()
    }
}

/// Single particle internal representation for fast WASM numerical integration.
#[allow(dead_code)]
#[derive(Clone, Debug)]
struct SwarmParticle {
    id: u64,
    name: String,
    rel_pos: [f64; 3],
    rel_vel: [f64; 3],
    sma: f64,
    ecc: f64,
    inc_rad: f64,
    period_s: f64,
    v_min: f64,
    v_max: f64,
    trail: Vec<[f64; 3]>,
}

/// Swarm particle propagation engine running in pure WebAssembly.
#[wasm_bindgen]
pub struct WasmCentricSwarmEngine {
    anchor_name: String,
    anchor_mass_kg: f64,
    anchor_radius_m: f64,
    anchor_r_eq_m: f64,
    anchor_j2: f64,
    perturbation_mode: u8,
    particles: Vec<SwarmParticle>,
}


#[wasm_bindgen]
impl WasmCentricSwarmEngine {
    /// Generates Keplerian swarm orbits around the specified anchor body.
    #[wasm_bindgen(constructor)]
    pub fn new(anchor_name: &str, preset: &str, count: usize) -> Result<WasmCentricSwarmEngine, JsValue> {
        let (anchor_mass_kg, anchor_radius_m) = match anchor_name {
            "Earth" => (5.972e24, 6371e3),
            "Moon" => (7.342e22, 1737e3),
            "Jupiter" => (1.898e27, 69911e3),
            "Venus" => (4.867e24, 6052e3),
            "Mars" => (6.417e23, 3389e3),
            _ => (1.989e30, 696340e3), // Sun
        };

        let (j2_r_eq, j2_val) = sbm_core::nbody::get_body_j2_parameters(anchor_name);
        let anchor_r_eq_m = if j2_r_eq > 0.0 { j2_r_eq } else { anchor_radius_m };
        let anchor_j2 = j2_val;
        let perturbation_mode = 2; // Default to FullPerturbed (3rd-Body + J2)

        let mu = G_STANDARD * anchor_mass_kg;
        let (r_min, r_max, base_ecc_min, base_ecc_max) = if anchor_name == "Earth" {
            if preset == "rings" {
                (45_000_000.0, 110_000_000.0, 0.0, 0.03)
            } else if preset == "cislunar" {
                (45_000_000.0, 70_000_000.0, 0.72, 0.90)
            } else {
                (48_000_000.0, 350_000_000.0, 0.02, 0.32)
            }
        } else if anchor_name == "Moon" {
            if preset == "rings" {
                (3_000_000.0, 10_000_000.0, 0.0, 0.04)
            } else {
                (3_500_000.0, 35_000_000.0, 0.02, 0.26)
            }
        } else if anchor_name == "Jupiter" {
            (800_000_000.0, 15_000_000_000.0, 0.02, 0.28)
        } else {
            (0.45 * 1.496e11, 4.5 * 1.496e11, 0.01, 0.20)
        };

        let mut particles = Vec::with_capacity(count);
        let mut rng_seed = (42 + count * 17) as u64;
        let mut rnd = || {
            rng_seed = (rng_seed.wrapping_mul(1664525).wrapping_add(1013904223)) % 4294967296;
            (rng_seed as f64) / 4294967296.0
        };

        for i in 0..count {
            let (rx, ry, rz, vx, vy, vz);
            if preset == "cislunar" && anchor_name == "Earth" {
                let r_peri = r_min + rnd() * (r_max - r_min);
                let r_apo = 330_000_000.0 + rnd() * 65_000_000.0;
                let a = (r_peri + r_apo) / 2.0;
                let v_peri = (2.0 * mu / r_peri - mu / a).max(0.0).sqrt();
                let theta = rnd() * 2.0 * PI;
                let inc = (rnd() - 0.5) * 0.12;

                rx = r_peri * theta.cos() * inc.cos();
                ry = r_peri * theta.sin() * inc.cos();
                rz = r_peri * inc.sin();

                vx = -v_peri * theta.sin() * inc.cos();
                vy = v_peri * theta.cos() * inc.cos();
                vz = (rnd() - 0.5) * v_peri * 0.06;
            } else {
                let frac = (i as f64 + rnd() * 0.5) / (count as f64);
                let r = r_min + frac * (r_max - r_min);
                let theta = rnd() * 2.0 * PI;
                let inc_angle = (rnd() - 0.5) * 0.32;

                rx = r * theta.cos() * inc_angle.cos();
                ry = r * theta.sin() * inc_angle.cos();
                rz = r * inc_angle.sin();

                let v_circ = (mu / r).sqrt();
                let ecc = base_ecc_min + rnd() * (base_ecc_max - base_ecc_min);
                let speed = v_circ * (1.0 + (rnd() - 0.5) * ecc);

                vx = -speed * theta.sin() * inc_angle.cos();
                vy = speed * theta.cos() * inc_angle.cos();
                vz = (rnd() - 0.5) * speed * 0.10;
            }

            let r_init = (rx * rx + ry * ry + rz * rz).sqrt().max(1.0);
            let v_init = (vx * vx + vy * vy + vz * vz).sqrt();
            let specific_energy = 0.5 * v_init * v_init - mu / r_init;
            let sma = if specific_energy.abs() > 1e-12 {
                -mu / (2.0 * specific_energy)
            } else {
                r_init
            };

            let hx = ry * vz - rz * vy;
            let hy = rz * vx - rx * vz;
            let hz = rx * vy - ry * vx;
            let h = (hx * hx + hy * hy + hz * hz).sqrt().max(1e-12);

            let vxh_x = vy * hz - vz * hy;
            let vxh_y = vz * hx - vx * hz;
            let vxh_z = vx * hy - vy * hx;

            let ex = vxh_x / mu - rx / r_init;
            let ey = vxh_y / mu - ry / r_init;
            let ez = vxh_z / mu - rz / r_init;
            let ecc_val = (ex * ex + ey * ey + ez * ez).sqrt().min(0.98);

            let inc_rad = (hz / h).clamp(-1.0, 1.0).acos();
            let period_s = if sma > 0.0 {
                2.0 * PI * (sma.powi(3) / mu).sqrt()
            } else {
                f64::INFINITY
            };

            let r_p = sma * (1.0 - ecc_val);
            let r_a = sma * (1.0 + ecc_val);
            let v_min = compute_vis_viva_speed(mu, r_a, sma);
            let v_max = compute_vis_viva_speed(mu, r_p, sma);

            // Pre-calculate closed Keplerian ellipse orbit trail (180 steps)
            let mut trail = Vec::with_capacity(181);
            let p_param = (h * h) / mu;
            let (px_unit, py_unit, pz_unit) = if ecc_val > 1e-4 {
                (ex / ecc_val, ey / ecc_val, ez / ecc_val)
            } else {
                (rx / r_init, ry / r_init, rz / r_init)
            };
            let (wx, wy, wz) = (hx / h, hy / h, hz / h);
            let (qx, qy, qz) = (wy * pz_unit - wz * py_unit, wz * px_unit - wx * pz_unit, wx * py_unit - wy * px_unit);

            let steps = 180;
            for s in 0..=steps {
                let nu = (s as f64 / steps as f64) * 2.0 * PI;
                let r_nu = p_param / (1.0 + ecc_val * nu.cos());
                let px = r_nu * (nu.cos() * px_unit + nu.sin() * qx);
                let py = r_nu * (nu.cos() * py_unit + nu.sin() * qy);
                let pz = r_nu * (nu.cos() * pz_unit + nu.sin() * qz);
                trail.push([px, py, pz]);
            }

            particles.push(SwarmParticle {
                id: (i + 1) as u64,
                name: format!("SAT-{:03}", i + 1),
                rel_pos: [rx, ry, rz],
                rel_vel: [vx, vy, vz],
                sma,
                ecc: ecc_val,
                inc_rad,
                period_s,
                v_min,
                v_max,
                trail,
            });
        }

        Ok(WasmCentricSwarmEngine {
            anchor_name: anchor_name.to_string(),
            anchor_mass_kg,
            anchor_radius_m,
            anchor_r_eq_m,
            anchor_j2,
            perturbation_mode,
            particles,
        })
    }

    /// Sets the active perturbation model:
    /// 0 = TwoBody (pure Keplerian)
    /// 1 = ThirdBody (central + lunisolar/planetary third-body tidal & reflex)
    /// 2 = FullPerturbed (central + third-body + J2 oblateness)
    pub fn set_perturbation_mode(&mut self, mode: u8) {
        self.perturbation_mode = mode.min(2);
    }

    /// Gets the active perturbation mode.
    pub fn get_perturbation_mode(&self) -> u8 {
        self.perturbation_mode
    }

    /// Numerically integrates all particles in the non-inertial relative centric frame.
    ///
    /// Supports pure two-body, lunisolar third-body, and oblate J2 zonal gravitational perturbations.
    pub fn step(&mut self, dt: f64, sub_steps: usize, perturbers_flat: &[f64]) {
        let mu = G_STANDARD * self.anchor_mass_kg;
        let sub_dt = dt / (sub_steps.max(1) as f64);
        let n_sub = sub_steps.max(1);

        // Pre-parse perturbers and compute indirect d'Alembert reflex accelerations
        let mut perturbers = Vec::new();
        let mut a_ind = [0.0, 0.0, 0.0];
        if self.perturbation_mode >= 1 && perturbers_flat.len() >= 4 {
            let num_p = perturbers_flat.len() / 4;
            for k in 0..num_p {
                let px = perturbers_flat[k * 4];
                let py = perturbers_flat[k * 4 + 1];
                let pz = perturbers_flat[k * 4 + 2];
                let pm = perturbers_flat[k * 4 + 3];
                let dist2 = px * px + py * py + pz * pz;
                let dist = dist2.sqrt();
                if dist > 1e3 && pm > 0.0 {
                    let mu_k = G_STANDARD * pm;
                    let factor_ind = -mu_k / (dist2 * dist);
                    a_ind[0] += factor_ind * px;
                    a_ind[1] += factor_ind * py;
                    a_ind[2] += factor_ind * pz;
                    perturbers.push((px, py, pz, mu_k));
                }
            }
        }

        let do_j2 = self.perturbation_mode == 2 && self.anchor_j2.abs() > 1e-15 && self.anchor_r_eq_m > 0.0;

        for _ in 0..n_sub {
            for p in &mut self.particles {
                let rx = p.rel_pos[0];
                let ry = p.rel_pos[1];
                let rz = p.rel_pos[2];
                let r2 = rx * rx + ry * ry + rz * rz;
                let r = r2.sqrt().max(1.0);
                let r3 = r2 * r;

                // 1. Central two-body acceleration
                let factor = -mu / r3;
                let mut ax = factor * rx;
                let mut ay = factor * ry;
                let mut az = factor * rz;

                // 2. Oblate zonal J2 perturbation
                if do_j2 {
                    let a_j2 = sbm_core::nbody::compute_j2_acceleration(
                        p.rel_pos, mu, self.anchor_r_eq_m, self.anchor_j2,
                    );
                    ax += a_j2[0];
                    ay += a_j2[1];
                    az += a_j2[2];
                }

                // 3. Third-body perturbations (direct + indirect reflex)
                if !perturbers.is_empty() {
                    ax += a_ind[0];
                    ay += a_ind[1];
                    az += a_ind[2];

                    for &(px, py, pz, mu_k) in &perturbers {
                        let delta_x = px - rx;
                        let delta_y = py - ry;
                        let delta_z = pz - rz;
                        let delta2 = delta_x * delta_x + delta_y * delta_y + delta_z * delta_z;
                        let delta = delta2.sqrt().max(1.0);
                        let f_dir = mu_k / (delta2 * delta);
                        ax += f_dir * delta_x;
                        ay += f_dir * delta_y;
                        az += f_dir * delta_z;
                    }
                }

                // Symplectic integration step
                p.rel_vel[0] += ax * sub_dt;
                p.rel_vel[1] += ay * sub_dt;
                p.rel_vel[2] += az * sub_dt;

                p.rel_pos[0] += p.rel_vel[0] * sub_dt;
                p.rel_pos[1] += p.rel_vel[1] * sub_dt;
                p.rel_pos[2] += p.rel_vel[2] * sub_dt;
            }
        }
    }


    /// Returns flat array of particle relative positions: [x0, y0, z0, x1, y1, z1, ...].
    pub fn get_positions_flat(&self) -> Float64Array {
        let mut flat = Vec::with_capacity(self.particles.len() * 3);
        for p in &self.particles {
            flat.extend_from_slice(&p.rel_pos);
        }
        Float64Array::from(flat.as_slice())
    }

    /// Returns flat array of normalized Vis-Viva kinetic parameters: tau in [0.0, 1.0].
    pub fn get_vis_viva_kinetic_flat(&self) -> Float64Array {
        let mut taus = Vec::with_capacity(self.particles.len());
        for p in &self.particles {
            let v = (p.rel_vel[0] * p.rel_vel[0] + p.rel_vel[1] * p.rel_vel[1] + p.rel_vel[2] * p.rel_vel[2]).sqrt();
            let tau = if p.v_max > p.v_min + 1e-4 {
                ((v - p.v_min) / (p.v_max - p.v_min)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            taus.push(tau);
        }
        Float64Array::from(taus.as_slice())
    }

    /// Returns flat array of the closed Keplerian ellipse orbit vertices for particle `index`.
    pub fn get_orbit_trail_flat(&self, index: usize) -> Float64Array {
        if index >= self.particles.len() {
            return Float64Array::new(&JsValue::from(0));
        }
        let p = &self.particles[index];
        let mut flat = Vec::with_capacity(p.trail.len() * 3);
        for pt in &p.trail {
            flat.extend_from_slice(pt);
        }
        Float64Array::from(flat.as_slice())
    }

    /// Evaluates eclipse state (0 = Sunlit, 1 = Penumbra, 2 = Umbra) for all particles.
    pub fn get_eclipse_states_flat(&self, sun_rel_pos: &[f64], sun_radius_m: f64) -> js_sys::Int32Array {
        let mut states = Vec::with_capacity(self.particles.len());
        if sun_rel_pos.len() < 3 {
            states.resize(self.particles.len(), 0);
            return js_sys::Int32Array::from(states.as_slice());
        }

        let geom_opt = compute_shadow_cone_geometry(
            [0.0, 0.0, 0.0],
            self.anchor_radius_m,
            [sun_rel_pos[0], sun_rel_pos[1], sun_rel_pos[2]],
            sun_radius_m,
        );

        if let Some(geom) = geom_opt {
            for p in &self.particles {
                let ecl = evaluate_eclipse_state(p.rel_pos, self.anchor_radius_m, &geom);
                let code = match ecl {
                    EclipseState::Sunlit => 0,
                    EclipseState::Penumbra => 1,
                    EclipseState::Umbra => 2,
                };
                states.push(code);
            }
        } else {
            states.resize(self.particles.len(), 0);
        }

        js_sys::Int32Array::from(states.as_slice())
    }

    /// Computes full satellite telemetry (Kepler elements, altitude, speed, period, eclipse, acceleration breakdown).
    pub fn get_satellite_telemetry(
        &self,
        index: usize,
        sun_rel_pos: &[f64],
        sun_radius_m: f64,
        perturbers_flat: &[f64],
    ) -> Result<JsValue, JsValue> {
        if index >= self.particles.len() {
            return Err(JsValue::from_str("Particle index out of bounds"));
        }
        let p = &self.particles[index];
        let anchor_body = CelestialBody {
            id: 0,
            name: self.anchor_name.clone(),
            mass_kg: self.anchor_mass_kg,
            radius_km: self.anchor_radius_m / 1e3,
            position_m: [0.0, 0.0, 0.0],
            velocity_mps: [0.0, 0.0, 0.0],
            color_hex: "#38bdf8".to_string(),
            is_fixed: false,
        };

        let eclipse_state = if sun_rel_pos.len() >= 3 {
            if let Some(geom) = compute_shadow_cone_geometry(
                [0.0, 0.0, 0.0],
                self.anchor_radius_m,
                [sun_rel_pos[0], sun_rel_pos[1], sun_rel_pos[2]],
                sun_radius_m,
            ) {
                evaluate_eclipse_state(p.rel_pos, self.anchor_radius_m, &geom)
            } else {
                EclipseState::Sunlit
            }
        } else {
            EclipseState::Sunlit
        };

        let mut tel = sbm_core::nbody::compute_satellite_orbital_telemetry(
            p.id,
            &p.name,
            &anchor_body,
            p.rel_pos,
            p.rel_vel,
            eclipse_state,
        );

        // Compute instantaneous acceleration breakdown
        let mu = G_STANDARD * self.anchor_mass_kg;
        let r2 = p.rel_pos[0] * p.rel_pos[0] + p.rel_pos[1] * p.rel_pos[1] + p.rel_pos[2] * p.rel_pos[2];
        let a_central_mag = if r2 > 1.0 { mu / r2 } else { 0.0 };

        let (a_j2_vec, a_j2_mag) = if self.perturbation_mode == 2 && self.anchor_j2.abs() > 1e-15 && self.anchor_r_eq_m > 0.0 {
            let vec = sbm_core::nbody::compute_j2_acceleration(
                p.rel_pos, mu, self.anchor_r_eq_m, self.anchor_j2,
            );
            let mag = (vec[0] * vec[0] + vec[1] * vec[1] + vec[2] * vec[2]).sqrt();
            (vec, mag)
        } else {
            ([0.0, 0.0, 0.0], 0.0)
        };

        let mut a_3rd_vec = [0.0, 0.0, 0.0];
        let mut max_perturber_pull = 0.0;
        let mut dominant_perturber_name = "None".to_string();

        if self.perturbation_mode >= 1 && perturbers_flat.len() >= 4 {
            let num_p = perturbers_flat.len() / 4;
            for k in 0..num_p {
                let px = perturbers_flat[k * 4];
                let py = perturbers_flat[k * 4 + 1];
                let pz = perturbers_flat[k * 4 + 2];
                let pm = perturbers_flat[k * 4 + 3];
                let dist2 = px * px + py * py + pz * pz;
                let dist = dist2.sqrt();
                if dist > 1e3 && pm > 0.0 {
                    let mu_k = G_STANDARD * pm;
                    // Indirect d'Alembert reflex
                    let f_ind = -mu_k / (dist2 * dist);
                    let ind_x = f_ind * px;
                    let ind_y = f_ind * py;
                    let ind_z = f_ind * pz;

                    // Direct
                    let dx = px - p.rel_pos[0];
                    let dy = py - p.rel_pos[1];
                    let dz = pz - p.rel_pos[2];
                    let d2 = dx * dx + dy * dy + dz * dz;
                    let d = d2.sqrt().max(1.0);
                    let f_dir = mu_k / (d2 * d);
                    let dir_x = f_dir * dx;
                    let dir_y = f_dir * dy;
                    let dir_z = f_dir * dz;

                    let net_k_x = dir_x + ind_x;
                    let net_k_y = dir_y + ind_y;
                    let net_k_z = dir_z + ind_z;
                    let pull = (net_k_x * net_k_x + net_k_y * net_k_y + net_k_z * net_k_z).sqrt();

                    if pull > max_perturber_pull {
                        max_perturber_pull = pull;
                        dominant_perturber_name = if pm > 1e29 {
                            "Sun".to_string()
                        } else if pm > 1e26 {
                            "Jupiter".to_string()
                        } else if pm > 5e24 {
                            "Earth".to_string()
                        } else if pm > 5e22 {
                            "Moon".to_string()
                        } else if pm > 5e23 {
                            "Mars".to_string()
                        } else {
                            "Third Body".to_string()
                        };
                    }

                    a_3rd_vec[0] += net_k_x;
                    a_3rd_vec[1] += net_k_y;
                    a_3rd_vec[2] += net_k_z;
                }
            }
        }

        let a_third_body_mag = (a_3rd_vec[0] * a_3rd_vec[0] + a_3rd_vec[1] * a_3rd_vec[1] + a_3rd_vec[2] * a_3rd_vec[2]).sqrt();
        let r = r2.sqrt().max(1.0);
        let a_central_vec = [-mu / (r2 * r) * p.rel_pos[0], -mu / (r2 * r) * p.rel_pos[1], -mu / (r2 * r) * p.rel_pos[2]];
        let a_tot_vec = [
            a_central_vec[0] + a_j2_vec[0] + a_3rd_vec[0],
            a_central_vec[1] + a_j2_vec[1] + a_3rd_vec[1],
            a_central_vec[2] + a_j2_vec[2] + a_3rd_vec[2],
        ];
        let a_total_mag = (a_tot_vec[0] * a_tot_vec[0] + a_tot_vec[1] * a_tot_vec[1] + a_tot_vec[2] * a_tot_vec[2]).sqrt();

        tel.acceleration = Some(sbm_core::nbody::ParticleAccelerationBreakdown {
            a_central_mps2: a_central_mag,
            a_j2_mps2: a_j2_mag,
            a_third_body_mps2: a_third_body_mag,
            a_total_mps2: a_total_mag,
            dominant_perturber_name,
        });

        serde_wasm_bindgen::to_value(&tel).map_err(|e| JsValue::from_str(&e.to_string()))
    }


    /// Returns the particle count in the swarm.
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

/// Computes spatial gravitational field point and tidal tensor in pure WebAssembly.
#[wasm_bindgen]
pub fn wasm_compute_spatial_field_point(
    engine: &WasmNBodyEngine,
    px: f64,
    py: f64,
    pz: f64,
) -> Result<JsValue, JsValue> {
    let pt = compute_spatial_field_point(&engine.system, [px, py, pz]);
    serde_wasm_bindgen::to_value(&pt).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Evaluates shadow cone geometry in pure WebAssembly.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn wasm_compute_shadow_cone(
    body_x: f64,
    body_y: f64,
    body_z: f64,
    body_radius_m: f64,
    sun_x: f64,
    sun_y: f64,
    sun_z: f64,
    sun_radius_m: f64,
) -> Result<JsValue, JsValue> {
    let geom = compute_shadow_cone_geometry([body_x, body_y, body_z], body_radius_m, [sun_x, sun_y, sun_z], sun_radius_m);
    serde_wasm_bindgen::to_value(&geom).map_err(|e| JsValue::from_str(&e.to_string()))
}
