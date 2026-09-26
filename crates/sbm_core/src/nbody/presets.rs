//! Standard astronomical and dynamical benchmark presets for N-body gravitational systems.
//!
//! Includes:
//! - **Solar System (JPL J2000)**: NASA JPL Horizons state vectors for Sun, 8 planets, Pluto, and Moon.
//! - **Jovian Laplace Resonance (4:2:1)**: Jupiter and the Galilean moons (Io, Europa, Ganymede, Callisto).
//! - **Figure-8 Choreography**: Exact 3-body zero-angular-momentum periodic solution (Chenciner & Montgomery 2000).
//! - **Sun-Jupiter Trojans (L4/L5 1:1 Resonance)**: Primary, secondary, and Trojan asteroid camps.
//! - **Relativistic Mercury Precession**: Sun-Mercury orbit with 1PN General Relativity active.
//! - **Pythagorean 3-Body Problem**: Chaotic benchmark with masses 3, 4, 5 (Burrau 1913).
//! - **TRAPPIST-1 Resonant Chain**: Multi-planet exoplanetary system in Laplace-like resonance chain.

use crate::nbody::types::{
    CelestialBody, IntegratorType, NBodySystem, ASTRONOMICAL_UNIT_M, G_STANDARD,
};

/// Preset type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetId {
    SolarSystemJpl,
    LaplaceResonance,
    FigureEight,
    SunJupiterTrojans,
    RelativisticMercury,
    Pythagorean3Body,
    Trappist1Chain,
    InnerSolarSystemJupiter,
}

impl PresetId {
    pub fn all() -> &'static [PresetId] {
        &[
            PresetId::InnerSolarSystemJupiter,
            PresetId::SolarSystemJpl,
            PresetId::LaplaceResonance,
            PresetId::FigureEight,
            PresetId::SunJupiterTrojans,
            PresetId::RelativisticMercury,
            PresetId::Pythagorean3Body,
            PresetId::Trappist1Chain,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PresetId::InnerSolarSystemJupiter => "Sun-Venus-Earth-Moon-Mars-Jupiter (Gravitational Force Focus)",
            PresetId::SolarSystemJpl => "Solar System (JPL J2000)",
            PresetId::LaplaceResonance => "Jovian Laplace Resonance (4:2:1)",
            PresetId::FigureEight => "Figure-8 Three-Body Choreography",
            PresetId::SunJupiterTrojans => "Sun-Jupiter Trojans (L4/L5 Resonance)",
            PresetId::RelativisticMercury => "Relativistic Mercury Precession (1PN)",
            PresetId::Pythagorean3Body => "Pythagorean Three-Body Problem (Burrau 1913)",
            PresetId::Trappist1Chain => "TRAPPIST-1 Exoplanetary Resonant Chain",
        }
    }
}

/// Generates an N-body system matching the requested preset.
pub fn create_preset(preset: PresetId) -> NBodySystem {
    match preset {
        PresetId::InnerSolarSystemJupiter => create_inner_solar_system_jupiter(),
        PresetId::SolarSystemJpl => create_solar_system_jpl(),
        PresetId::LaplaceResonance => create_laplace_resonance_jovian(),
        PresetId::FigureEight => create_figure_eight_choreography(),
        PresetId::SunJupiterTrojans => create_sun_jupiter_trojans(),
        PresetId::RelativisticMercury => create_relativistic_mercury(),
        PresetId::Pythagorean3Body => create_pythagorean_three_body(),
        PresetId::Trappist1Chain => create_trappist1_resonant_chain(),
    }
}

/// Inner Solar System + Jupiter: Focus on gravitational force interactions,
/// Earth-Moon barycenter, and Jovian secular perturbations.
pub fn create_inner_solar_system_jupiter() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida4th;
    let au = ASTRONOMICAL_UNIT_M;

    // 0: Sun (1.98847e30 kg)
    system.add_body(CelestialBody::new(
        0,
        "Sun",
        1.98847e30,
        696340.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#fbbf24",
    ));

    // 1: Venus (0.723 AU, mass: 4.8675e24 kg)
    system.add_body(CelestialBody::new(
        1,
        "Venus",
        4.8675e24,
        6051.8,
        [0.721 * au, 0.088 * au, -0.038 * au],
        [-4.2e3, 34.8e3, 1.2e3],
        "#f59e0b",
    ));

    // 2: Earth (1.000 AU, mass: 5.9722e24 kg)
    system.add_body(CelestialBody::new(
        2,
        "Earth",
        5.9722e24,
        6371.0,
        [-0.178 * au, 0.967 * au, -0.0001 * au],
        [-29.8e3, -5.2e3, 0.0],
        "#38bdf8",
    ));

    // 3: Moon (384,400 km from Earth, mass: 7.342e22 kg)
    system.add_body(CelestialBody::new(
        3,
        "Moon",
        7.342e22,
        1737.4,
        [-0.178 * au + 384400e3, 0.967 * au, 28000e3],
        [-29.8e3, -5.2e3 + 1022.0, 90.0],
        "#cbd5e1",
    ));

    // 4: Mars (1.524 AU, mass: 6.4171e23 kg)
    system.add_body(CelestialBody::new(
        4,
        "Mars",
        6.4171e23,
        3389.5,
        [1.385 * au, -0.635 * au, -0.045 * au],
        [10.1e3, 22.4e3, 0.2e3],
        "#ef4444",
    ));

    // 5: Jupiter (5.204 AU, mass: 1.89813e27 kg)
    system.add_body(CelestialBody::new(
        5,
        "Jupiter",
        1.89813e27,
        69911.0,
        [4.020 * au, 3.290 * au, -0.105 * au],
        [-8.4e3, 10.7e3, 0.2e3],
        "#fb923c",
    ));

    // Barycentric momentum balance
    let mut px = 0.0;
    let mut py = 0.0;
    let mut pz = 0.0;
    for b in &system.bodies[1..] {
        px += b.mass_kg * b.velocity_mps[0];
        py += b.mass_kg * b.velocity_mps[1];
        pz += b.mass_kg * b.velocity_mps[2];
    }
    system.bodies[0].velocity_mps = [
        -px / system.bodies[0].mass_kg,
        -py / system.bodies[0].mass_kg,
        -pz / system.bodies[0].mass_kg,
    ];

    system
}

/// Solar System with Sun, 8 planets, Pluto, and Moon at J2000.0 epoch from NASA JPL Horizons.
pub fn create_solar_system_jpl() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida4th;
    let au = ASTRONOMICAL_UNIT_M;

    // Sun (mass: 1.98847e30 kg, radius: 696,340 km)
    system.add_body(CelestialBody::new(
        0,
        "Sun",
        1.98847e30,
        696340.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#fbbf24",
    ));

    // Mercury (0.387 AU, e=0.2056, mass: 3.3011e23 kg)
    system.add_body(CelestialBody::new(
        1,
        "Mercury",
        3.3011e23,
        2439.7,
        [-0.370 * au, -0.125 * au, 0.015 * au],
        [10.2e3, -44.5e3, -4.8e3],
        "#94a3b8",
    ));

    // Venus (0.723 AU, e=0.0068, mass: 4.8675e24 kg)
    system.add_body(CelestialBody::new(
        2,
        "Venus",
        4.8675e24,
        6051.8,
        [0.721 * au, 0.088 * au, -0.038 * au],
        [-4.2e3, 34.8e3, 1.2e3],
        "#f59e0b",
    ));

    // Earth (1.000 AU, mass: 5.9722e24 kg)
    let earth_pos = [-0.178 * au, 0.967 * au, -0.0001 * au];
    let earth_vel = [-29.8e3, -5.2e3, 0.0];
    system.add_body(CelestialBody::new(
        3,
        "Earth",
        5.9722e24,
        6371.0,
        earth_pos,
        earth_vel,
        "#38bdf8",
    ));

    // Moon (Earth's moon, 384,400 km, mass: 7.342e22 kg)
    system.add_body(CelestialBody::new(
        4,
        "Moon",
        7.342e22,
        1737.4,
        [
            earth_pos[0] + 384400e3,
            earth_pos[1],
            earth_pos[2] + 28000e3,
        ],
        [
            earth_vel[0],
            earth_vel[1] + 1022.0,
            earth_vel[2] + 90.0,
        ],
        "#cbd5e1",
    ));

    // Mars (1.524 AU, e=0.0934, mass: 6.4171e23 kg)
    system.add_body(CelestialBody::new(
        5,
        "Mars",
        6.4171e23,
        3389.5,
        [1.385 * au, -0.635 * au, -0.045 * au],
        [10.1e3, 22.4e3, 0.2e3],
        "#ef4444",
    ));

    // Jupiter (5.204 AU, mass: 1.89813e27 kg)
    system.add_body(CelestialBody::new(
        6,
        "Jupiter",
        1.89813e27,
        69911.0,
        [4.020 * au, 3.290 * au, -0.105 * au],
        [-8.4e3, 10.7e3, 0.2e3],
        "#fb923c",
    ));

    // Saturn (9.582 AU, mass: 5.6834e26 kg)
    system.add_body(CelestialBody::new(
        7,
        "Saturn",
        5.6834e26,
        58232.0,
        [6.420 * au, -6.950 * au, -0.220 * au],
        [6.8e3, 6.7e3, -0.3e3],
        "#facc15",
    ));

    // Uranus (19.22 AU, mass: 8.6810e25 kg)
    system.add_body(CelestialBody::new(
        8,
        "Uranus",
        8.6810e25,
        25362.0,
        [14.30 * au, 12.80 * au, -0.180 * au],
        [-4.5e3, 4.8e3, 0.08e3],
        "#2dd4bf",
    ));

    // Neptune (30.07 AU, mass: 1.02413e26 kg)
    system.add_body(CelestialBody::new(
        9,
        "Neptune",
        1.02413e26,
        24622.0,
        [28.10 * au, -10.40 * au, -0.450 * au],
        [1.8e3, 5.1e3, -0.15e3],
        "#60a5fa",
    ));

    // Pluto (39.48 AU, e=0.2488, mass: 1.303e22 kg)
    system.add_body(CelestialBody::new(
        10,
        "Pluto",
        1.303e22,
        1188.3,
        [-13.50 * au, -28.20 * au, 6.20 * au],
        [5.2e3, -1.9e3, -1.5e3],
        "#a855f7",
    ));

    // Adjust Sun's velocity so system barycentric momentum is strictly zero
    let mut px = 0.0;
    let mut py = 0.0;
    let mut pz = 0.0;
    for b in &system.bodies[1..] {
        px += b.mass_kg * b.velocity_mps[0];
        py += b.mass_kg * b.velocity_mps[1];
        pz += b.mass_kg * b.velocity_mps[2];
    }
    system.bodies[0].velocity_mps = [
        -px / system.bodies[0].mass_kg,
        -py / system.bodies[0].mass_kg,
        -pz / system.bodies[0].mass_kg,
    ];

    system
}

/// Jovian system with Jupiter and the 4 Galilean moons exhibiting the 4:2:1 Laplace resonance.
pub fn create_laplace_resonance_jovian() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida4th;

    // Jupiter at origin
    let mj = 1.89813e27;
    system.add_body(CelestialBody::new(
        0,
        "Jupiter",
        mj,
        69911.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#fb923c",
    ));

    // Io: a = 421,700 km, orbital period = 1.769 days, speed = 17,334 m/s
    let r1 = 421700e3;
    let v1 = (G_STANDARD * mj / r1).sqrt();
    system.add_body(CelestialBody::new(
        1,
        "Io",
        8.9319e22,
        1821.6,
        [r1, 0.0, 0.0],
        [0.0, v1, 0.0],
        "#facc15",
    ));

    // Europa: a = 670,900 km, orbital period = 3.551 days (~2x Io), speed = 13,740 m/s
    // In Laplace resonance, when Io and Ganymede are in conjunction (0 deg), Europa is in opposition (180 deg)
    let r2 = 670900e3;
    let v2 = (G_STANDARD * mj / r2).sqrt();
    system.add_body(CelestialBody::new(
        2,
        "Europa",
        4.7998e22,
        1560.8,
        [-r2, 0.0, 0.0],
        [0.0, -v2, 0.0],
        "#e2e8f0",
    ));

    // Ganymede: a = 1,070,400 km, orbital period = 7.155 days (~4x Io), speed = 10,880 m/s
    // Initialized at 0 deg (conjunction with Io) -> phi_L = 0 - 3*(180) + 2*(0) = -540 deg = 180 deg
    let r3 = 1070400e3;
    let v3 = (G_STANDARD * mj / r3).sqrt();
    system.add_body(CelestialBody::new(
        3,
        "Ganymede",
        1.4819e23,
        2634.1,
        [r3, 0.0, 0.0],
        [0.0, v3, 0.0],
        "#94a3b8",
    ));

    // Callisto: a = 1,882,700 km, orbital period = 16.689 days, speed = 8,204 m/s
    let r4 = 1882700e3;
    let v4 = (G_STANDARD * mj / r4).sqrt();
    system.add_body(CelestialBody::new(
        4,
        "Callisto",
        1.0759e23,
        2410.3,
        [0.0, -r4, 0.0],
        [v4, 0.0, 0.0],
        "#64748b",
    ));

    system
}

/// Exact Figure-8 3-body choreography (Chenciner & Montgomery 2000, Simó 2002).
pub fn create_figure_eight_choreography() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida4th;

    // Physical scaling: equal masses of 1.0e24 kg, characteristic scale 1.0e8 m
    let m = 1.0e24;
    let r_scale = 1.0e8;
    // Scale velocity such that G * M / R aligns with dimensionless choreography parameters
    let v_scale = (G_STANDARD * m / r_scale).sqrt();

    let x1 = -0.97000436 * r_scale;
    let y1 = 0.24308753 * r_scale;

    let vx3 = -0.93240737 * v_scale;
    let vy3 = -0.86473146 * v_scale;
    let vx1 = -0.5 * vx3;
    let vy1 = -0.5 * vy3;

    system.add_body(CelestialBody::new(
        0,
        "Body Alpha",
        m,
        2000.0,
        [x1, y1, 0.0],
        [vx1, vy1, 0.0],
        "#38bdf8",
    ));

    system.add_body(CelestialBody::new(
        1,
        "Body Beta",
        m,
        2000.0,
        [-x1, -y1, 0.0],
        [vx1, vy1, 0.0],
        "#f43f5e",
    ));

    system.add_body(CelestialBody::new(
        2,
        "Body Gamma",
        m,
        2000.0,
        [0.0, 0.0, 0.0],
        [vx3, vy3, 0.0],
        "#a855f7",
    ));

    system
}

/// Sun-Jupiter system with Trojan asteroid swarms at L4 (+60 deg) and L5 (-60 deg).
pub fn create_sun_jupiter_trojans() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida4th;

    let ms = 1.98847e30;
    let mj = 1.89813e27;
    let rj = 5.2044 * ASTRONOMICAL_UNIT_M;
    let vj = (G_STANDARD * (ms + mj) / rj).sqrt();

    // Sun
    system.add_body(CelestialBody::new(
        0,
        "Sun",
        ms,
        696340.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#fbbf24",
    ));

    // Jupiter on circular orbit
    system.add_body(CelestialBody::new(
        1,
        "Jupiter",
        mj,
        69911.0,
        [rj, 0.0, 0.0],
        [0.0, vj, 0.0],
        "#fb923c",
    ));

    // L4 Greek Camp (60 deg ahead of Jupiter: cos(60) = 0.5, sin(60) = sqrt(3)/2)
    let cos60 = 0.5;
    let sin60 = (3.0_f64).sqrt() * 0.5;
    let m_trojan = 1.0e16; // Small asteroid mass

    system.add_body(CelestialBody::new(
        2,
        "Achilles (L4 Greek)",
        m_trojan,
        135.0,
        [rj * cos60, rj * sin60, 0.0],
        [-vj * sin60, vj * cos60, 0.0],
        "#34d399",
    ));

    system.add_body(CelestialBody::new(
        3,
        "Hektor (L4 Greek)",
        m_trojan,
        112.0,
        [rj * (cos60 * 1.01), rj * (sin60 * 0.99), 0.02 * rj],
        [-vj * sin60 * 0.99, vj * cos60 * 1.01, 100.0],
        "#10b981",
    ));

    // L5 Trojan Camp (60 deg behind Jupiter: cos(-60) = 0.5, sin(-60) = -sqrt(3)/2)
    system.add_body(CelestialBody::new(
        4,
        "Patroclus (L5 Trojan)",
        m_trojan,
        140.0,
        [rj * cos60, -rj * sin60, 0.0],
        [vj * sin60, vj * cos60, 0.0],
        "#c084fc",
    ));

    system.add_body(CelestialBody::new(
        5,
        "Priamus (L5 Trojan)",
        m_trojan,
        100.0,
        [rj * (cos60 * 0.98), -rj * (sin60 * 1.02), -0.01 * rj],
        [vj * sin60 * 1.02, vj * cos60 * 0.98, -150.0],
        "#a855f7",
    ));

    system
}

/// Relativistic Sun-Mercury system demonstrating the 1PN perihelion advance.
pub fn create_relativistic_mercury() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida4th;
    system.enable_general_relativity = true;

    let ms = 1.98847e30;
    let mm = 3.3011e23;
    let a = 0.387098 * ASTRONOMICAL_UNIT_M;
    let e = 0.205630;

    // Perihelion state
    let r_peri = a * (1.0 - e);
    let mu = G_STANDARD * (ms + mm);
    let v_peri = (mu * (2.0 / r_peri - 1.0 / a)).sqrt();

    system.add_body(CelestialBody::new(
        0,
        "Sun",
        ms,
        696340.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#fbbf24",
    ));

    system.add_body(CelestialBody::new(
        1,
        "Mercury",
        mm,
        2439.7,
        [r_peri, 0.0, 0.0],
        [0.0, v_peri, 0.0],
        "#94a3b8",
    ));

    system
}

/// Pythagorean 3-Body Problem (Burrau 1913, Szebehely 1967).
pub fn create_pythagorean_three_body() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida6th;
    // Plummer softening to handle ultra-close binary encounters smoothly
    system.softening_m = 1.0e6;

    let m_unit = 1.0e26; // kg
    let l_unit = 1.0e9; // meters

    // Body 1: mass 3 at (1, 3)
    system.add_body(CelestialBody::new(
        0,
        "Body 1 (Mass 3)",
        3.0 * m_unit,
        4000.0,
        [1.0 * l_unit, 3.0 * l_unit, 0.0],
        [0.0, 0.0, 0.0],
        "#ef4444",
    ));

    // Body 2: mass 4 at (-2, -1)
    system.add_body(CelestialBody::new(
        1,
        "Body 2 (Mass 4)",
        4.0 * m_unit,
        5000.0,
        [-2.0 * l_unit, -l_unit, 0.0],
        [0.0, 0.0, 0.0],
        "#3b82f6",
    ));

    // Body 3: mass 5 at (1, -1)
    system.add_body(CelestialBody::new(
        2,
        "Body 3 (Mass 5)",
        5.0 * m_unit,
        6000.0,
        [1.0 * l_unit, -l_unit, 0.0],
        [0.0, 0.0, 0.0],
        "#22c55e",
    ));

    system
}

/// TRAPPIST-1 ultra-cool red dwarf system with 7 resonant planets (b through h).
pub fn create_trappist1_resonant_chain() -> NBodySystem {
    let mut system = NBodySystem::new();
    system.integrator = IntegratorType::Yoshida4th;

    // Star: 0.0898 Solar masses, radius 0.121 Solar radii
    let m_star = 0.0898 * 1.98847e30;
    system.add_body(CelestialBody::new(
        0,
        "TRAPPIST-1",
        m_star,
        84157.0,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "#f87171",
    ));

    let au = ASTRONOMICAL_UNIT_M;
    let m_earth = 5.9722e24;

    // Semi-major axes and masses of planets b, c, d, e, f, g, h
    let planets = [
        ("TRAPPIST-1b", 1.017 * m_earth, 1.121 * 6371.0, 0.01154 * au, "#f97316"),
        ("TRAPPIST-1c", 1.156 * m_earth, 1.095 * 6371.0, 0.01580 * au, "#fbbf24"),
        ("TRAPPIST-1d", 0.297 * m_earth, 0.784 * 6371.0, 0.02227 * au, "#34d399"),
        ("TRAPPIST-1e", 0.772 * m_earth, 0.910 * 6371.0, 0.02925 * au, "#38bdf8"),
        ("TRAPPIST-1f", 0.934 * m_earth, 1.046 * 6371.0, 0.03849 * au, "#818cf8"),
        ("TRAPPIST-1g", 1.148 * m_earth, 1.148 * 6371.0, 0.04683 * au, "#c084fc"),
        ("TRAPPIST-1h", 0.331 * m_earth, 0.773 * 6371.0, 0.06189 * au, "#f472b6"),
    ];

    for (idx, &(name, mass, radius, a, color)) in planets.iter().enumerate() {
        let v = (G_STANDARD * (m_star + mass) / a).sqrt();
        system.add_body(CelestialBody::new(
            idx + 1,
            name,
            mass,
            radius,
            [a, 0.0, 0.0],
            [0.0, v, 0.0],
            color,
        ));
    }

    system
}
