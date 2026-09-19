//! Domain types and data models for NASA EVOLVE 4.0 breakup simulation.

/// Object type classification for EVOLVE 4.0 area-to-mass distributions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    /// Payload / Satellite / Spacecraft (characterized by solar panels, payloads, bus structures).
    Spacecraft,
    /// Upper stage / Rocket Body (characterized by cylindrical pressure tanks, propellant shells, nozzles).
    RocketBody,
}

/// Breakup event regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakupType {
    /// Hypervelocity impact between two celestial objects or satellites.
    Collision,
    /// Internal energetic rupture (propellant explosion, battery rupture, kinetic failure).
    Explosion,
}

/// A single debris fragment produced by a breakup event under NASA EVOLVE 4.0.
#[derive(Debug, Clone, PartialEq)]
pub struct Fragment {
    /// Unique identifier within the sampled fragment population.
    pub id: usize,
    /// Characteristic length $L_c$ in meters (average of 3 orthogonal dimensions).
    pub size_m: f64,
    /// Average cross-sectional area $A_x$ in $\text{m}^2$ (Johnson et al. 2001, Eqs. 8 & 9).
    pub cross_section_m2: f64,
    /// Area-to-mass ratio $A/M$ in $\text{m}^2/\text{kg}$ (Johnson et al. 2001, Eqs. 5, 6, 7).
    pub am_ratio: f64,
    /// Logarithmic Area-to-Mass ratio $\chi = \log_{10}(A/M)$.
    pub chi: f64,
    /// Physical fragment mass in kilograms (Johnson et al. 2001, Eq. 10).
    pub mass_kg: f64,
    /// Ejection velocity magnitude $\Delta v$ in meters per second (Eqs. 11 & 12).
    pub speed_mps: f64,
    /// 3D relative ejection velocity vector $[\Delta v_x, \Delta v_y, \Delta v_z]$ in m/s (LVLH / Hill frame).
    pub vel_vec: [f64; 3],
    /// Parent origin tag (0 = Target satellite, 1 = Impactor projectile).
    pub origin: u8,
    /// Parent object class (Spacecraft or RocketBody).
    pub object_type: ObjectType,
    /// Speed contour classification band for telemetry and visualization.
    pub contour_band: &'static str,
}

impl Fragment {
    /// Calculates the ballistic coefficient $B^* = \frac{1}{2} C_D (A/M)$ in $\text{m}^2/\text{kg}$.
    /// If `drag_coeff` is `None`, the standard hypersonic debris value $C_D = 2.2$ is used.
    pub fn ballistic_coefficient(&self, drag_coeff: Option<f64>) -> f64 {
        let cd = drag_coeff.unwrap_or(2.2);
        0.5 * cd * self.am_ratio
    }

    /// Calculates the kinetic energy of this fragment relative to the parent center of mass:
    /// $E_k = \frac{1}{2} m \Delta v^2$ in Joules.
    pub fn kinetic_energy_joules(&self) -> f64 {
        0.5 * self.mass_kg * self.speed_mps.powi(2)
    }
}

/// Comprehensive outcome of a NASA EVOLVE 4.0 breakup simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakupResult {
    /// Discrete sampled fragments generated for analysis and visualization.
    pub fragments: Vec<Fragment>,
    /// Whether the collision exceeded the catastrophic disruption threshold ($E_p \ge 40.0\text{ kJ/kg}$).
    pub is_catastrophic: bool,
    /// Specific impact energy $E_p$ in $\text{kJ/kg}$.
    pub specific_energy_kj_per_kg: f64,
    /// Total mass converted into fragmented debris in kilograms.
    pub destroyed_mass_kg: f64,
    /// Surviving intact parent remnant mass in kilograms (cratering regime only).
    pub remnant_mass_kg: f64,
    /// Theoretical physical fragment yield for $L_c \ge 1\text{ cm}$ (Johnson et al. 2001, Eq. 4 or 3).
    pub physical_yield_1cm: f64,
    /// Theoretical Space Surveillance Network (SSN) trackable yield for $L_c \ge 10\text{ cm}$.
    pub ssn_trackable_yield_10cm: f64,
}

impl BreakupResult {
    /// Returns the $N$ heaviest fragments sorted by mass descending.
    pub fn top_heaviest(&self, n: usize) -> Vec<&Fragment> {
        let mut refs: Vec<&Fragment> = self.fragments.iter().collect();
        refs.sort_by(|a, b| b.mass_kg.partial_cmp(&a.mass_kg).unwrap_or(std::cmp::Ordering::Equal));
        refs.truncate(n);
        refs
    }

    /// Returns the $N$ fastest fragments sorted by ejection velocity descending.
    pub fn top_fastest(&self, n: usize) -> Vec<&Fragment> {
        let mut refs: Vec<&Fragment> = self.fragments.iter().collect();
        refs.sort_by(|a, b| b.speed_mps.partial_cmp(&a.speed_mps).unwrap_or(std::cmp::Ordering::Equal));
        refs.truncate(n);
        refs
    }

    /// Calculates the sum of all sampled fragment masses in kilograms.
    pub fn total_fragment_mass(&self) -> f64 {
        self.fragments.iter().map(|f| f.mass_kg).sum()
    }

    /// Serializes the sampled fragments to standard JSON without requiring external dependencies.
    pub fn to_fragments_json(&self) -> String {
        let mut out = String::with_capacity(self.fragments.len() * 180 + 16);
        out.push_str("[\n");
        for (i, f) in self.fragments.iter().enumerate() {
            let comma = if i + 1 < self.fragments.len() { "," } else { "" };
            out.push_str(&format!(
                "  {{\"id\":{},\"size\":{:.6},\"area\":{:.8},\"am_ratio\":{:.4},\"chi\":{:.4},\"mass\":{:.8},\"speed\":{:.1},\"contour\":\"{}\",\"vx\":{:.1},\"vy\":{:.1},\"vz\":{:.1}}}{}\n",
                f.id, f.size_m, f.cross_section_m2, f.am_ratio, f.chi, f.mass_kg, f.speed_mps, f.contour_band, f.vel_vec[0], f.vel_vec[1], f.vel_vec[2], comma
            ));
        }
        out.push_str("]\n");
        out
    }
}
