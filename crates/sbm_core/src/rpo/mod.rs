//! # Rendezvous & Proximity Operations (RPO) Engine
//!
//! Grounded in classical Hill-Clohessy-Wiltshire (CW) relative dynamics:
//! > **Clohessy, W. H., & Wiltshire, R. S. (1960).**  
//! > *Terminal Guidance System for Satellite Rendezvous.*  
//! > Journal of the Aerospace Sciences, 27(9), pp. 653–658.
//!
//! Features:
//! - **Analytical CW State Transition Matrix (STM)**: High-speed exact 6x6 propagation in the Local-Vertical Local-Horizontal (LVLH) frame.
//! - **Targeted Two-Impulse Transfers (CW Targeting)**: Computes optimal departure $\Delta \mathbf{v}_1$ and arrival braking $\Delta \mathbf{v}_2$ to reach any waypoint in specified flight time.
//! - **Natural Motion Circumnavigation (NMC)**: Closed periodic elliptical relative inspection orbits with **zero propellant consumption** ($\dot{y}_0 + 2\omega x_0 = 0$).
//! - **V-Bar & R-Bar Glideslope Approaches**: Controlled approaches along the velocity vector ($V$-bar) or radial direction ($R$-bar, fail-safe passive abort).

pub mod cw;
pub mod types;

pub use cw::{
    cw_state_transition_matrix, invert_3x3, plan_glideslope_rbar, plan_glideslope_vbar,
    plan_natural_motion_circumnavigation, plan_two_impulse_transfer, propagate_cw,
};
pub use types::{
    GlideslopeApproachPlan, NmcInspectionPlan, RelativeState, RpoError, RpoManeuverDto,
    TargetOrbit, TwoImpulseTransferPlan,
};
