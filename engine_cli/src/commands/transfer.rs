//! CR3BP invariant manifold low-energy transfer CLI command.

use sbm_core::cr3bp::{compute_earth_moon_l1_to_l2_transfer, Cr3bpSystem};
use tracing::info;

pub fn run_transfer_cli(_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sys = Cr3bpSystem::earth_moon();
    info!("============================================================");
    info!(" CR3BP LOW-ENERGY INVARIANT MANIFOLD TRANSFER (CLI)");
    info!("============================================================");
    info!(
        system = "Earth-Moon",
        mass_ratio_mu = sys.mu,
        characteristic_length_km = sys.l_star,
        characteristic_time_days = sys.t_star,
        characteristic_velocity_mps = sys.v_star,
        "System parameters loaded"
    );

    let transfer = compute_earth_moon_l1_to_l2_transfer(&sys, None)
        .map_err(|e| format!("{:?}", e))?;

    info!(
        departure_orbit = "Earth-Moon L1 Southern Lyapunov",
        target_orbit = "Earth-Moon L2 Southern Lyapunov",
        dv1_mps = transfer.dv1_ms,
        dv2_mps = transfer.dv2_ms,
        dv3_mps = transfer.dv3_ms,
        total_delta_v_mps = transfer.total_dv_ms,
        pos_match_error_m = transfer.position_match_error_m,
        vel_match_error_mps = transfer.velocity_match_error_ms,
        "Low-energy heteroclinic transfer evaluated (AAS 20-459)"
    );
    info!("============================================================");

    Ok(())
}
