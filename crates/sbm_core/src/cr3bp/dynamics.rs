//! CR3BP equations of motion, pseudo-potential, variational matrix, and Jacobi constant.
//!
//! Grounded in AAS 20-459:
//! > **Short, C., Haapala, A., & Bosanac, N. (2020).**  
//! > *Technical Implementation of the Circular Restricted Three-Body Model in STK Astrogator.*  
//! > AAS/AIAA Astrodynamics Specialist Conference, AAS 20-459.
//!
//! Equations (1)–(3), (10):
//! - Pseudo-potential: $U^* = \frac{1-\mu}{r_{13}} + \frac{\mu}{r_{23}} + \frac{1}{2}(x^2 + y^2)$
//! - Equations of Motion: $\ddot{x} - 2\dot{y} = U^*_x$, $\ddot{y} + 2\dot{x} = U^*_y$, $\ddot{z} = U^*_z$
//! - Jacobi Integral: $C_J = 2U^* - v^2$
//! - Variational Matrix: $A_{ND}^R(t) = \begin{bmatrix} 0 & I \\ U^*_{xx} & 2\Omega \end{bmatrix}$

use crate::cr3bp::types::{Cr3bpState, Cr3bpState9D, Cr3bpSystem};

/// Evaluates distances $r_{13}$ and $r_{23}$ from the third body to primary $P_1$ and secondary $P_2$.
///
/// In the nondimensional rotating barycentric frame:
/// - Primary $P_1$ is at $(-\mu, 0, 0)$
/// - Secondary $P_2$ is at $(1-\mu, 0, 0)$
///
/// $$r_{13} = \sqrt{(x + \mu)^2 + y^2 + z^2}$$
/// $$r_{23} = \sqrt{(x - 1 + \mu)^2 + y^2 + z^2}$$
#[inline]
pub fn distances_to_primaries(mu: f64, x: f64, y: f64, z: f64) -> (f64, f64) {
    let y2_z2 = y * y + z * z;
    let dx1 = x + mu;
    let dx2 = x - 1.0 + mu;
    let r13 = (dx1 * dx1 + y2_z2).sqrt();
    let r23 = (dx2 * dx2 + y2_z2).sqrt();
    (r13, r23)
}

/// Evaluates the pseudo-potential $U^*(x, y, z)$ (Equation 2 in AAS 20-459).
///
/// $$U^* = \frac{1-\mu}{r_{13}} + \frac{\mu}{r_{23}} + \frac{1}{2}(x^2 + y^2)$$
pub fn pseudo_potential(system: &Cr3bpSystem, x: f64, y: f64, z: f64) -> f64 {
    let (r13, r23) = distances_to_primaries(system.mu, x, y, z);
    if r13 == 0.0 || r23 == 0.0 {
        return f64::INFINITY;
    }
    let mu = system.mu;
    (1.0 - mu) / r13 + mu / r23 + 0.5 * (x * x + y * y)
}

/// Evaluates the gradient of the pseudo-potential $\nabla U^* = [U^*_x, U^*_y, U^*_z]^T$.
///
/// $$U^*_x = x - \frac{(1-\mu)(x+\mu)}{r_{13}^3} - \frac{\mu(x-1+\mu)}{r_{23}^3}$$
/// $$U^*_y = y - \frac{(1-\mu)y}{r_{13}^3} - \frac{\mu y}{r_{23}^3}$$
/// $$U^*_z = -\frac{(1-\mu)z}{r_{13}^3} - \frac{\mu z}{r_{23}^3}$$
pub fn pseudo_potential_gradient(system: &Cr3bpSystem, x: f64, y: f64, z: f64) -> [f64; 3] {
    let (r13, r23) = distances_to_primaries(system.mu, x, y, z);
    let mu = system.mu;
    let r13_3 = r13 * r13 * r13;
    let r23_3 = r23 * r23 * r23;

    let term1 = (1.0 - mu) / r13_3;
    let term2 = mu / r23_3;

    let ux = x - term1 * (x + mu) - term2 * (x - 1.0 + mu);
    let uy = y - term1 * y - term2 * y;
    let uz = -term1 * z - term2 * z;

    [ux, uy, uz]
}

/// Evaluates the 3x3 Hessian matrix of second spatial partial derivatives of the pseudo-potential $U^*_{ij}$.
///
/// Matrix components:
/// $$U^*_{xx} = 1 - \frac{1-\mu}{r_{13}^3} - \frac{\mu}{r_{23}^3} + \frac{3(1-\mu)(x+\mu)^2}{r_{13}^5} + \frac{3\mu(x-1+\mu)^2}{r_{23}^5}$$
/// $$U^*_{yy} = 1 - \frac{1-\mu}{r_{13}^3} - \frac{\mu}{r_{23}^3} + \frac{3(1-\mu)y^2}{r_{13}^5} + \frac{3\mu y^2}{r_{23}^5}$$
/// $$U^*_{zz} = -\frac{1-\mu}{r_{13}^3} - \frac{\mu}{r_{23}^3} + \frac{3(1-\mu)z^2}{r_{13}^5} + \frac{3\mu z^2}{r_{23}^5}$$
/// $$U^*_{xy} = U^*_{yx} = \frac{3(1-\mu)(x+\mu)y}{r_{13}^5} + \frac{3\mu(x-1+\mu)y}{r_{23}^5}$$
/// $$U^*_{xz} = U^*_{zx} = \frac{3(1-\mu)(x+\mu)z}{r_{13}^5} + \frac{3\mu(x-1+\mu)z}{r_{23}^5}$$
/// $$U^*_{yz} = U^*_{zy} = \frac{3(1-\mu)y z}{r_{13}^5} + \frac{3\mu y z}{r_{23}^5}$$
pub fn pseudo_potential_hessian(system: &Cr3bpSystem, x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
    let (r13, r23) = distances_to_primaries(system.mu, x, y, z);
    let mu = system.mu;
    let r13_3 = r13 * r13 * r13;
    let r23_3 = r23 * r23 * r23;
    let r13_5 = r13_3 * r13 * r13;
    let r23_5 = r23_3 * r23 * r23;

    let dx1 = x + mu;
    let dx2 = x - 1.0 + mu;

    let term1_3 = (1.0 - mu) / r13_3;
    let term2_3 = mu / r23_3;
    let term1_5_3 = 3.0 * (1.0 - mu) / r13_5;
    let term2_5_3 = 3.0 * mu / r23_5;

    let uxx = 1.0 - term1_3 - term2_3 + term1_5_3 * dx1 * dx1 + term2_5_3 * dx2 * dx2;
    let uyy = 1.0 - term1_3 - term2_3 + term1_5_3 * y * y + term2_5_3 * y * y;
    let uzz = -term1_3 - term2_3 + term1_5_3 * z * z + term2_5_3 * z * z;

    let uxy = term1_5_3 * dx1 * y + term2_5_3 * dx2 * y;
    let uxz = term1_5_3 * dx1 * z + term2_5_3 * dx2 * z;
    let uyz = term1_5_3 * y * z + term2_5_3 * y * z;

    [[uxx, uxy, uxz], [uxy, uyy, uyz], [uxz, uyz, uzz]]
}

/// Evaluates the 6D equations of motion $\dot{\mathbf{x}} = f(\mathbf{x})$ (Equation 1 in AAS 20-459).
///
/// $$\dot{x} = v_x$$
/// $$\dot{y} = v_y$$
/// $$\dot{z} = v_z$$
/// $$\dot{v}_x = 2 v_y + U^*_x$$
/// $$\dot{v}_y = -2 v_x + U^*_y$$
/// $$\dot{v}_z = U^*_z$$
pub fn equations_of_motion(system: &Cr3bpSystem, state: &Cr3bpState) -> [f64; 6] {
    let grad = pseudo_potential_gradient(system, state.x, state.y, state.z);
    let ax = 2.0 * state.vy + grad[0];
    let ay = -2.0 * state.vx + grad[1];
    let az = grad[2];

    [state.vx, state.vy, state.vz, ax, ay, az]
}

/// Evaluates the 9-dimensional state $[x, y, z, v_x, v_y, v_z, a_x, a_y, a_z]^T$.
pub fn equations_of_motion_9d(system: &Cr3bpSystem, state: &Cr3bpState) -> Cr3bpState9D {
    let deriv = equations_of_motion(system, state);
    Cr3bpState9D::new(*state, deriv[3], deriv[4], deriv[5])
}

/// Evaluates the 6x6 nondimensional variational matrix $A_{ND}^R(t)$ (Equation 10 in AAS 20-459).
///
/// $$A_{ND}^R = \begin{bmatrix} \mathbf{0}_{3\times 3} & \mathbf{I}_{3\times 3} \\ \mathbf{U}^*_{xx} & 2\mathbf{\Omega} \end{bmatrix}$$
///
/// where $2\mathbf{\Omega} = \begin{bmatrix} 0 & 2 & 0 \\ -2 & 0 & 0 \\ 0 & 0 & 0 \end{bmatrix}$.
pub fn variational_matrix(system: &Cr3bpSystem, x: f64, y: f64, z: f64) -> [[f64; 6]; 6] {
    let hess = pseudo_potential_hessian(system, x, y, z);
    let mut a = [[0.0; 6]; 6];

    // Upper right 3x3 is Identity
    a[0][3] = 1.0;
    a[1][4] = 1.0;
    a[2][5] = 1.0;

    // Lower left 3x3 is Hessian of U*
    for i in 0..3 {
        for j in 0..3 {
            a[3 + i][j] = hess[i][j];
        }
    }

    // Lower right 3x3 is 2 * Omega (Coriolis terms)
    a[3][4] = 2.0;
    a[4][3] = -2.0;

    a
}

/// Evaluates the full 42-element derivative vector:
/// - Elements 0..6: 6D state derivative $[\dot{x}, \dot{y}, \dot{z}, \dot{v}_x, \dot{v}_y, \dot{v}_z]$
/// - Elements 6..42: 36 elements of the State Transition Matrix derivative $\dot{\mathbf{\Phi}} = \mathbf{A}\mathbf{\Phi}$
///
/// The STM $\mathbf{\Phi}$ is stored in row-major order: index $6 + 6i + j$ holds $\Phi_{ij}$.
pub fn state_and_stm_derivatives(system: &Cr3bpSystem, y_42: &[f64; 42]) -> [f64; 42] {
    let state = Cr3bpState::new(y_42[0], y_42[1], y_42[2], y_42[3], y_42[4], y_42[5]);
    let state_deriv = equations_of_motion(system, &state);
    let a = variational_matrix(system, state.x, state.y, state.z);

    let mut out = [0.0; 42];
    out[..6].copy_from_slice(&state_deriv);

    // dPhi = A * Phi
    // Phi_kj = y_42[6 + 6*k + j]
    for i in 0..6 {
        for j in 0..6 {
            let mut sum = 0.0;
            for k in 0..6 {
                let phi_kj = y_42[6 + 6 * k + j];
                sum += a[i][k] * phi_kj;
            }
            out[6 + 6 * i + j] = sum;
        }
    }

    out
}

/// Evaluates the Jacobi integral of motion $C_J$ (Equation 3 in AAS 20-459).
///
/// $$C_J = 2U^* - v^2$$
///
/// Along any unforced, exact trajectory in the CR3BP, $C_J$ remains strictly constant: $\Delta C_J = 0$.
pub fn jacobi_constant(system: &Cr3bpSystem, state: &Cr3bpState) -> f64 {
    let u_star = pseudo_potential(system, state.x, state.y, state.z);
    let v_sq = state.v_squared();
    2.0 * u_star - v_sq
}

/// Checks if a position $(x, y, z)$ is physically admissible (kinematically allowed) for a given Jacobi constant $C_J$.
///
/// Motion is only permitted where $2 U^*(x, y, z) \ge C_J$ (since $v^2 = 2 U^* - C_J \ge 0$).
/// Points where $2 U^* < C_J$ lie inside the "forbidden region".
pub fn is_region_accessible(system: &Cr3bpSystem, x: f64, y: f64, z: f64, cj: f64) -> bool {
    let u_star = pseudo_potential(system, x, y, z);
    2.0 * u_star >= cj
}

/// Computes the maximum squared velocity $v^2 = 2 U^* - C_J$ allowed at $(x, y, z)$ for energy level $C_J$.
/// Returns `None` if the point is within the forbidden region ($2 U^* < C_J$).
pub fn allowed_velocity_squared(system: &Cr3bpSystem, x: f64, y: f64, z: f64, cj: f64) -> Option<f64> {
    let u_star = pseudo_potential(system, x, y, z);
    let v2 = 2.0 * u_star - cj;
    if v2 >= 0.0 {
        Some(v2)
    } else {
        None
    }
}
