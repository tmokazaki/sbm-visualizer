//! Planetary shadow cone geometry (umbra and penumbra) and solar eclipse state evaluation.

use crate::nbody::types::{EclipseState, ShadowConeGeometry};

/// Computes the conical shadow geometry (umbra and penumbra cones) for an occulting body illuminated by the Sun.
pub fn compute_shadow_cone_geometry(
    body_pos_m: [f64; 3],
    body_radius_m: f64,
    sun_pos_m: [f64; 3],
    sun_radius_m: f64,
) -> Option<ShadowConeGeometry> {
    let dx = body_pos_m[0] - sun_pos_m[0];
    let dy = body_pos_m[1] - sun_pos_m[1];
    let dz = body_pos_m[2] - sun_pos_m[2];
    let d_sun = (dx * dx + dy * dy + dz * dz).sqrt();

    if d_sun <= 1e-6 || sun_radius_m <= body_radius_m {
        return None;
    }

    let shadow_axis_unit = [dx / d_sun, dy / d_sun, dz / d_sun];
    let delta_r = sun_radius_m - body_radius_m;
    let umbra_length_m = (body_radius_m * d_sun) / delta_r;
    let tan_umbra = delta_r / d_sun;
    let tan_penumbra = (sun_radius_m + body_radius_m) / d_sun;

    Some(ShadowConeGeometry {
        umbra_length_m,
        umbra_half_angle_rad: tan_umbra.atan(),
        penumbra_half_angle_rad: tan_penumbra.atan(),
        shadow_axis_unit,
    })
}

/// Evaluates whether a point (e.g. satellite relative to body center) is Sunlit, in Penumbra, or in Umbra.
pub fn evaluate_eclipse_state(
    rel_pos_m: [f64; 3],
    body_radius_m: f64,
    shadow_geom: &ShadowConeGeometry,
) -> EclipseState {
    let axis = shadow_geom.shadow_axis_unit;
    // Distance downstream along the shadow axis
    let x = rel_pos_m[0] * axis[0] + rel_pos_m[1] * axis[1] + rel_pos_m[2] * axis[2];

    // If point is on the day-side (towards the Sun), it is completely sunlit
    if x <= 0.0 {
        return EclipseState::Sunlit;
    }

    // Perpendicular distance from shadow axis
    let perp_x = rel_pos_m[0] - x * axis[0];
    let perp_y = rel_pos_m[1] - x * axis[1];
    let perp_z = rel_pos_m[2] - x * axis[2];
    let rho = (perp_x * perp_x + perp_y * perp_y + perp_z * perp_z).sqrt();

    // Umbra radius at distance x
    let r_umbra = if x < shadow_geom.umbra_length_m {
        body_radius_m * (1.0 - x / shadow_geom.umbra_length_m)
    } else {
        0.0
    };

    if rho <= r_umbra {
        return EclipseState::Umbra;
    }

    // Penumbra radius at distance x
    let r_penumbra = body_radius_m + x * shadow_geom.penumbra_half_angle_rad.tan();
    if rho <= r_penumbra {
        return EclipseState::Penumbra;
    }

    EclipseState::Sunlit
}
