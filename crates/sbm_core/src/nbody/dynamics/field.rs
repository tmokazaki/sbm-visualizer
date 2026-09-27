//! Spatial gravitational field evaluation, dominance basins, and boundary spheres.

use crate::nbody::dynamics::tidal::compute_tidal_tensor;
use crate::nbody::types::{
    BodyFieldContribution, GravitationalSphereRadii, NBodySystem, SpatialFieldPoint,
};

/// Computes the planetary gravitational domains of dominance:
/// - Sphere of Attraction ($r_a = a \sqrt{m/M_\odot}$)
/// - Laplace Sphere of Influence ($r_s = a (m/M_\odot)^{2/5}$)
/// - Hill Sphere ($r_H = a(1-e) \sqrt[3]{m/(3M_\odot)}$)
/// - Domingos et al. (2006) Critical Satellite Stability Radius ($r_{\text{crit}} \approx 0.4895 r_H$).
pub fn compute_gravitational_spheres(
    body_mass_kg: f64,
    primary_mass_kg: f64,
    semimajor_axis_m: f64,
    eccentricity: f64,
    sat_eccentricity: f64,
) -> GravitationalSphereRadii {
    let mass_ratio = body_mass_kg / primary_mass_kg;
    let sphere_of_attraction_m = semimajor_axis_m * mass_ratio.sqrt();
    let laplace_soi_m = semimajor_axis_m * mass_ratio.powf(0.4);
    let hill_sphere_m = semimajor_axis_m * (1.0 - eccentricity) * (mass_ratio / 3.0).cbrt();
    let critical_stability_radius_m = 0.4895 * hill_sphere_m * (1.0 - 1.0305 * sat_eccentricity - 0.2738 * eccentricity);

    GravitationalSphereRadii {
        sphere_of_attraction_m,
        laplace_soi_m,
        hill_sphere_m,
        critical_stability_radius_m,
    }
}

/// Evaluates the Earth-Moon Barycenter (EMB) position and displacement from Earth's center in meters.
pub fn compute_earth_moon_barycenter(system: &NBodySystem) -> Result<([f64; 3], f64), &'static str> {
    let earth = system.bodies.iter().find(|b| b.name.eq_ignore_ascii_case("earth"))
        .ok_or("Earth not found in system")?;
    let moon = system.bodies.iter().find(|b| b.name.eq_ignore_ascii_case("moon"))
        .ok_or("Moon not found in system")?;

    let total_mass = earth.mass_kg + moon.mass_kg;
    let emb = [
        (earth.position_m[0] * earth.mass_kg + moon.position_m[0] * moon.mass_kg) / total_mass,
        (earth.position_m[1] * earth.mass_kg + moon.position_m[1] * moon.mass_kg) / total_mass,
        (earth.position_m[2] * earth.mass_kg + moon.position_m[2] * moon.mass_kg) / total_mass,
    ];

    let dx = emb[0] - earth.position_m[0];
    let dy = emb[1] - earth.position_m[1];
    let dz = emb[2] - earth.position_m[2];
    let displacement_m = (dx * dx + dy * dy + dz * dz).sqrt();

    Ok((emb, displacement_m))
}

/// Evaluates the complete gravitational field state at an arbitrary spatial coordinate $\mathbf{r} = [x, y, z]$.
///
/// Computes:
/// - Net Newtonian gravitational acceleration vector $\mathbf{g}(\mathbf{r}) = \sum_{j=1}^N \frac{G m_j (\mathbf{r}_j - \mathbf{r})}{\|\mathbf{r}_j - \mathbf{r}\|^3}$
/// - Gravitational potential $\Phi(\mathbf{r}) = -\sum_{j=1}^N \frac{G m_j}{\|\mathbf{r}_j - \mathbf{r}\|}$
/// - Dominant gravitational body basin ($\arg\max_j \|\mathbf{g}_j(\mathbf{r})\|$)
/// - Individual body contributions (Tug-of-War breakdown at this spatial point)
/// - Gravitational tidal tensor $\mathbf{T}_{ab}(\mathbf{r}) = \partial g_a / \partial x_b$ and principal strain eigenvalues
pub fn compute_spatial_field_point(system: &NBodySystem, point_m: [f64; 3]) -> SpatialFieldPoint {
    let mut net_accel = [0.0, 0.0, 0.0];
    let mut total_potential = 0.0;
    let mut contributions = Vec::with_capacity(system.bodies.len());
    let mut scalar_sum = 0.0;
    let eps2 = system.softening_m * system.softening_m;

    let mut max_body_accel = -1.0;
    let mut dominant_id = 0;
    let mut dominant_name = String::new();

    for b in &system.bodies {
        let dx = b.position_m[0] - point_m[0];
        let dy = b.position_m[1] - point_m[1];
        let dz = b.position_m[2] - point_m[2];
        let r2 = dx * dx + dy * dy + dz * dz;
        let dist = r2.sqrt();
        let dist_soft_sq = r2 + eps2;
        let dist_soft = dist_soft_sq.sqrt();

        let (ax, ay, az, a_mag, phi) = if dist_soft > 1e-12 {
            let gm = system.gravitational_constant * b.mass_kg;
            let denom = dist_soft_sq * dist_soft;
            let factor = gm / denom;
            let ax = factor * dx;
            let ay = factor * dy;
            let az = factor * dz;
            let a_mag = (ax * ax + ay * ay + az * az).sqrt();
            let phi = -gm / dist_soft;
            (ax, ay, az, a_mag, phi)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        };

        net_accel[0] += ax;
        net_accel[1] += ay;
        net_accel[2] += az;
        total_potential += phi;
        scalar_sum += a_mag;

        if a_mag > max_body_accel {
            max_body_accel = a_mag;
            dominant_id = b.id;
            dominant_name = b.name.clone();
        }

        contributions.push(BodyFieldContribution {
            body_id: b.id,
            body_name: b.name.clone(),
            body_color: b.color_hex.clone(),
            acceleration_vector_mps2: [ax, ay, az],
            acceleration_magnitude: a_mag,
            gravitational_potential_j_kg: phi,
            fraction_of_total: 0.0,
            distance_m: dist,
        });
    }

    if scalar_sum > 1e-30 {
        for c in &mut contributions {
            c.fraction_of_total = c.acceleration_magnitude / scalar_sum;
        }
    }

    let dominant_body_fraction = if scalar_sum > 1e-30 {
        (max_body_accel / scalar_sum).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let net_mag = (net_accel[0] * net_accel[0] + net_accel[1] * net_accel[1] + net_accel[2] * net_accel[2]).sqrt();
    let tidal_tensor = compute_tidal_tensor(system, point_m);

    SpatialFieldPoint {
        position_m: point_m,
        acceleration_vector_mps2: net_accel,
        acceleration_magnitude: net_mag,
        gravitational_potential_j_kg: total_potential,
        dominant_body_id: dominant_id,
        dominant_body_name: dominant_name,
        dominant_body_fraction,
        contributions,
        tidal_tensor,
    }
}

/// Samples the gravitational field across a 2D bounding box on the orbital plane ($z = 0$).
pub fn compute_spatial_field_grid(
    system: &NBodySystem,
    x_range_m: [f64; 2],
    y_range_m: [f64; 2],
    resolution_x: usize,
    resolution_y: usize,
) -> Vec<SpatialFieldPoint> {
    let nx = resolution_x.max(2);
    let ny = resolution_y.max(2);
    let mut grid = Vec::with_capacity(nx * ny);

    let dx = (x_range_m[1] - x_range_m[0]) / ((nx - 1) as f64);
    let dy = (y_range_m[1] - y_range_m[0]) / ((ny - 1) as f64);

    for j in 0..ny {
        let y = y_range_m[0] + (j as f64) * dy;
        for i in 0..nx {
            let x = x_range_m[0] + (i as f64) * dx;
            grid.push(compute_spatial_field_point(system, [x, y, 0.0]));
        }
    }

    grid
}

/// Determines if a celestial body is relevant to the active gravitational centric frame.
pub fn is_body_relevant_to_centric(body_name: &str, centric_body_name: &str) -> bool {
    if centric_body_name.eq_ignore_ascii_case("Sun") {
        return true;
    }
    if centric_body_name.eq_ignore_ascii_case("Earth") {
        return body_name.eq_ignore_ascii_case("Earth") || body_name.eq_ignore_ascii_case("Moon");
    }
    if centric_body_name.eq_ignore_ascii_case("Moon") {
        return body_name.eq_ignore_ascii_case("Moon") || body_name.eq_ignore_ascii_case("Earth");
    }
    if centric_body_name.eq_ignore_ascii_case("Jupiter") {
        return body_name.eq_ignore_ascii_case("Jupiter")
            || body_name.eq_ignore_ascii_case("Io")
            || body_name.eq_ignore_ascii_case("Europa")
            || body_name.eq_ignore_ascii_case("Ganymede")
            || body_name.eq_ignore_ascii_case("Callisto");
    }
    body_name.eq_ignore_ascii_case(centric_body_name)
}

/// Computes a high-resolution gravitational field grid centered dynamically on a specific celestial body.
pub fn compute_centric_spatial_field_grid(
    system: &NBodySystem,
    center_body_name: &str,
    half_span_m: f64,
    resolution: usize,
) -> Result<Vec<SpatialFieldPoint>, String> {
    let center_body = system
        .bodies
        .iter()
        .find(|b| b.name.eq_ignore_ascii_case(center_body_name))
        .ok_or_else(|| format!("Body '{}' not found in system", center_body_name))?;

    let cx = center_body.position_m[0];
    let cy = center_body.position_m[1];
    let cz = center_body.position_m[2];

    let x_range = [cx - half_span_m, cx + half_span_m];
    let y_range = [cy - half_span_m, cy + half_span_m];

    let nx = resolution.max(2);
    let ny = resolution.max(2);
    let mut grid = Vec::with_capacity(nx * ny);

    let dx = (x_range[1] - x_range[0]) / ((nx - 1) as f64);
    let dy = (y_range[1] - y_range[0]) / ((ny - 1) as f64);

    let active_system;
    let sys_ref = if !center_body_name.eq_ignore_ascii_case("Sun") {
        active_system = NBodySystem {
            bodies: system
                .bodies
                .iter()
                .filter(|b| is_body_relevant_to_centric(&b.name, center_body_name))
                .cloned()
                .collect(),
            ..system.clone()
        };
        &active_system
    } else {
        system
    };

    for j in 0..ny {
        let y = y_range[0] + (j as f64) * dy;
        for i in 0..nx {
            let x = x_range[0] + (i as f64) * dx;
            grid.push(compute_spatial_field_point(sys_ref, [x, y, cz]));
        }
    }

    Ok(grid)
}
