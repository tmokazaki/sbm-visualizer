//! Frame transformations between dimensional Central Body Inertial (CBI) and nondimensional Rotating Barycentric frames.
//!
//! Grounded in AAS 20-459 Table 1 and Equations (4)–(11):
//! - Sequence CBI (D) $\rightarrow$ Rotating (ND):
//!   1. Rotate: $X_D^R(t) = R_{RI}(t) X_D^I(t)$
//!   2. Scale: $X_{ND}^R(t) = R_{SI}(t) X_D^R(t)$
//!   3. Shift: $x_{ND}^R(t) = R_{HI}(t) + X_{ND}^R(t)$
//! - Reverse Sequence Rotating (ND) $\rightarrow$ CBI (D):
//!   1. Shift: $\tilde{X}_{ND}^R(t) = I_{HR}(t) + \tilde{x}_{ND}^R(t)$
//!   2. Scale: $\tilde{X}_D^R(t) = I_{SR}(t) \tilde{X}_{ND}^R(t)$
//!   3. Rotate: $\tilde{X}_D^I(t) = I_{RR}(t) \tilde{X}_D^R(t)$

use crate::cr3bp::types::{Cr3bpState, Cr3bpState9D, Cr3bpSystem};

/// 3x3 matrix multiplication helper.
#[inline]
pub fn mat3_mul_vec3(m: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// 3x3 matrix product $C = A \cdot B$.
#[inline]
pub fn mat3_mul_mat3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    c
}

/// 6x6 matrix product $C = A \cdot B$.
pub fn mat6_mul_mat6(a: &[[f64; 6]; 6], b: &[[f64; 6]; 6]) -> [[f64; 6]; 6] {
    let mut c = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            let mut sum = 0.0;
            for k in 0..6 {
                sum += a[i][k] * b[k][j];
            }
            c[i][j] = sum;
        }
    }
    c
}

/// Computes the 3x3 direction cosine matrix (DCM) from Central Body Inertial (CBI) to Rotating frame $C_{R/I}(\theta)$.
///
/// In the standard CR3BP definition, the primary-secondary orbital plane is the $x-y$ plane.
/// $\theta(t) = \theta_0 + n t$ where $n$ is the mean motion of the primaries.
///
/// $$C_{R/I} = \begin{bmatrix} \cos\theta & \sin\theta & 0 \\ -\sin\theta & \cos\theta & 0 \\ 0 & 0 & 1 \end{bmatrix}$$
pub fn dcm_inertial_to_rotating(theta: f64) -> [[f64; 3]; 3] {
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    [
        [cos_t, sin_t, 0.0],
        [-sin_t, cos_t, 0.0],
        [0.0, 0.0, 1.0],
    ]
}

/// Computes the 3x3 direction cosine matrix (DCM) from Rotating frame to Central Body Inertial (CBI) $C_{I/R}(\theta) = C_{R/I}^T$.
pub fn dcm_rotating_to_inertial(theta: f64) -> [[f64; 3]; 3] {
    let cos_t = theta.cos();
    let sin_t = theta.sin();
    [
        [cos_t, -sin_t, 0.0],
        [sin_t, cos_t, 0.0],
        [0.0, 0.0, 1.0],
    ]
}

/// CR3BP Frame Transformer handling conversions between dimensional CBI and nondimensional Rotating Barycentric frames.
#[derive(Debug, Clone)]
pub struct FrameTransformer {
    pub system: Cr3bpSystem,
    /// Initial rotation angle $\theta_0$ of the secondary at epoch $t_0 = 0$ (radians). Default: 0.0.
    pub theta_0: f64,
}

impl FrameTransformer {
    /// Creates a new frame transformer for the given CR3BP system with $\theta_0 = 0$.
    pub fn new(system: Cr3bpSystem) -> Self {
        Self {
            system,
            theta_0: 0.0,
        }
    }

    /// Creates a frame transformer with a specific initial angle $\theta_0$.
    pub fn with_initial_angle(system: Cr3bpSystem, theta_0: f64) -> Self {
        Self { system, theta_0 }
    }

    /// Mean motion $n$ in rad/s: $n = 1 / t^*$.
    #[inline]
    pub fn mean_motion(&self) -> f64 {
        1.0 / self.system.t_star
    }

    /// Primary-secondary orbital rotation angle at dimensional time $t$ (seconds).
    #[inline]
    pub fn rotation_angle(&self, t_dim: f64) -> f64 {
        self.theta_0 + self.mean_motion() * t_dim
    }

    /// 6x6 kinematic rotation matrix $R_{RI}(t)$ from CBI to Rotating frame.
    ///
    /// $$R_{RI} = \begin{bmatrix} C_{R/I} & \mathbf{0} \\ -\mathbf{\Omega} C_{R/I} & C_{R/I} \end{bmatrix}$$
    ///
    /// where $\mathbf{\Omega} = \begin{bmatrix} 0 & -n & 0 \\ n & 0 & 0 \\ 0 & 0 & 0 \end{bmatrix}$.
    pub fn r_ri_6x6(&self, t_dim: f64) -> [[f64; 6]; 6] {
        let theta = self.rotation_angle(t_dim);
        let c_ri = dcm_inertial_to_rotating(theta);
        let n = self.mean_motion();

        let mut r = [[0.0; 6]; 6];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = c_ri[i][j];
                r[3 + i][3 + j] = c_ri[i][j];
            }
        }

        // -Omega * C_RI where Omega * v = [-n*vy, n*vx, 0]
        // -Omega = [ [0, n, 0], [-n, 0, 0], [0, 0, 0] ]
        // (-Omega * C_RI)[0][j] = n * c_ri[1][j]
        // (-Omega * C_RI)[1][j] = -n * c_ri[0][j]
        for j in 0..3 {
            r[3][j] = n * c_ri[1][j];
            r[4][j] = -n * c_ri[0][j];
        }

        r
    }

    /// 6x6 kinematic rotation matrix $I_{RR}(t)$ from Rotating frame to CBI.
    ///
    /// $$I_{RR} = \begin{bmatrix} C_{I/R} & \mathbf{0} \\ C_{I/R} \mathbf{\Omega} & C_{I/R} \end{bmatrix}$$
    pub fn i_rr_6x6(&self, t_dim: f64) -> [[f64; 6]; 6] {
        let theta = self.rotation_angle(t_dim);
        let c_ir = dcm_rotating_to_inertial(theta);
        let n = self.mean_motion();

        let mut r = [[0.0; 6]; 6];
        for i in 0..3 {
            for j in 0..3 {
                r[i][j] = c_ir[i][j];
                r[3 + i][3 + j] = c_ir[i][j];
            }
        }

        // C_IR * Omega where Omega = [ [0, -n, 0], [n, 0, 0], [0, 0, 0] ]
        // (C_IR * Omega)[i][0] = c_ir[i][1] * n
        // (C_IR * Omega)[i][1] = -c_ir[i][0] * n
        for i in 0..3 {
            r[3 + i][0] = c_ir[i][1] * n;
            r[3 + i][1] = -c_ir[i][0] * n;
        }

        r
    }

    /// Transforms a 6D dimensional CBI state vector to a nondimensional CR3BP Rotating Barycentric state vector.
    ///
    /// Implements AAS 20-459 Equations (4)–(6):
    /// 1. Rotate: $X_D^R(t) = R_{RI}(t) X_D^I(t)$
    /// 2. Scale: $X_{ND}^R(t) = R_{SI}(t) X_D^R(t)$
    /// 3. Shift: $x_{ND}^R(t) = R_{HI}(t) + X_{ND}^R(t)$
    pub fn cbi_to_rotating_6d(&self, state_cbi_dim: &Cr3bpState, t_dim: f64) -> Cr3bpState {
        let theta = self.rotation_angle(t_dim);
        let c_ri = dcm_inertial_to_rotating(theta);
        let n = self.mean_motion();

        // Step 1: Rotate
        let r_cbi = state_cbi_dim.position();
        let v_cbi = state_cbi_dim.velocity();

        let r_rot_d = mat3_mul_vec3(&c_ri, &r_cbi);
        // v_rot_d = C_RI * v_cbi - Omega * r_rot_d = C_RI * v_cbi - [-n*y_r, n*x_r, 0]
        let c_v = mat3_mul_vec3(&c_ri, &v_cbi);
        let v_rot_d = [
            c_v[0] + n * r_rot_d[1],
            c_v[1] - n * r_rot_d[0],
            c_v[2],
        ];

        // Step 2: Scale
        let r_rot_nd = self.system.nondimensionalize_position(r_rot_d);
        let v_rot_nd = self.system.nondimensionalize_velocity(v_rot_d);

        // Step 3: Shift
        // Primary center is at x = -mu. Spacecraft relative to barycenter is x_rot_nd - mu
        let x_bary = r_rot_nd[0] - self.system.mu;

        Cr3bpState::new(x_bary, r_rot_nd[1], r_rot_nd[2], v_rot_nd[0], v_rot_nd[1], v_rot_nd[2])
    }

    /// Transforms a nondimensional CR3BP Rotating Barycentric state vector to a dimensional CBI state vector.
    ///
    /// Implements AAS 20-459 Equations (7)–(9):
    /// 1. Shift: $\tilde{X}_{ND}^R(t) = I_{HR}(t) + \tilde{x}_{ND}^R(t)$
    /// 2. Scale: $\tilde{X}_D^R(t) = I_{SR}(t) \tilde{X}_{ND}^R(t)$
    /// 3. Rotate: $\tilde{X}_D^I(t) = I_{RR}(t) \tilde{X}_D^R(t)$
    pub fn rotating_to_cbi_6d(&self, state_rot_nd: &Cr3bpState, t_dim: f64) -> Cr3bpState {
        let theta = self.rotation_angle(t_dim);
        let c_ir = dcm_rotating_to_inertial(theta);
        let n = self.mean_motion();

        // Step 1: Shift to primary center
        let x_prim_nd = state_rot_nd.x + self.system.mu;
        let r_prim_nd = [x_prim_nd, state_rot_nd.y, state_rot_nd.z];
        let v_prim_nd = state_rot_nd.velocity();

        // Step 2: Scale to dimensional
        let r_rot_d = self.system.dimensionalize_position(r_prim_nd);
        let v_rot_d = self.system.dimensionalize_velocity(v_prim_nd);

        // Step 3: Rotate to CBI
        let r_cbi = mat3_mul_vec3(&c_ir, &r_rot_d);
        // v_cbi = C_IR * (v_rot_d + Omega * r_rot_d) where Omega * r_rot_d = [-n*y_rot_d, n*x_rot_d, 0]
        let v_sum = [
            v_rot_d[0] - n * r_rot_d[1],
            v_rot_d[1] + n * r_rot_d[0],
            v_rot_d[2],
        ];
        let v_cbi = mat3_mul_vec3(&c_ir, &v_sum);

        Cr3bpState::new(r_cbi[0], r_cbi[1], r_cbi[2], v_cbi[0], v_cbi[1], v_cbi[2])
    }

    /// Transforms 9D state (position, velocity, acceleration) from Rotating ND to CBI Dimensional.
    ///
    /// Implements AAS 20-459 Table 1 9x9 transformation.
    pub fn rotating_to_cbi_9d(&self, state_rot_nd: &Cr3bpState9D, t_dim: f64) -> Cr3bpState9D {
        let state_6d = self.rotating_to_cbi_6d(&state_rot_nd.state, t_dim);
        let theta = self.rotation_angle(t_dim);
        let c_ir = dcm_rotating_to_inertial(theta);
        let n = self.mean_motion();

        // Shifted & dimensionalized position and velocity in rotating frame
        let x_prim_nd = state_rot_nd.state.x + self.system.mu;
        let r_rot_d = self.system.dimensionalize_position([x_prim_nd, state_rot_nd.state.y, state_rot_nd.state.z]);
        let v_rot_d = self.system.dimensionalize_velocity(state_rot_nd.state.velocity());
        let a_rot_d = [
            state_rot_nd.ax * self.system.a_star,
            state_rot_nd.ay * self.system.a_star,
            state_rot_nd.az * self.system.a_star,
        ];

        // Kinematics in rotating frame:
        // a_inertial = C_IR * ( a_rot + 2 * Omega * v_rot + Omega^2 * r_rot )
        // Omega * v_rot = [-n*vy, n*vx, 0]
        // Omega^2 * r_rot = [-n^2*x, -n^2*y, 0]
        let a_sum = [
            a_rot_d[0] - 2.0 * n * v_rot_d[1] - n * n * r_rot_d[0],
            a_rot_d[1] + 2.0 * n * v_rot_d[0] - n * n * r_rot_d[1],
            a_rot_d[2],
        ];
        let a_cbi = mat3_mul_vec3(&c_ir, &a_sum);

        Cr3bpState9D::new(state_6d, a_cbi[0], a_cbi[1], a_cbi[2])
    }

    /// Transforms the 6x6 State Transition Matrix (STM) from Rotating frame to CBI frame (Equation 11).
    ///
    /// $$\mathbf{\Phi}_{I}(t, t_0) = I_{RR}(t) \mathbf{\Phi}_{R}(t, t_0) R_{RI}(t_0)$$
    pub fn transform_stm_rotating_to_cbi(
        &self,
        stm_rot: &[[f64; 6]; 6],
        t_dim: f64,
        t0_dim: f64,
    ) -> [[f64; 6]; 6] {
        let i_rr = self.i_rr_6x6(t_dim);
        let r_ri = self.r_ri_6x6(t0_dim);
        let temp = mat6_mul_mat6(&i_rr, stm_rot);
        mat6_mul_mat6(&temp, &r_ri)
    }
}
