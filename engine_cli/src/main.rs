use std::f64::consts::PI;
use std::fs::File;
use std::io::Write;

/// Simple, zero-dependency PRNG (Xorshift64)
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0xdeadbeef } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn rand_normal(&mut self) -> f64 {
        let mut u = 0.0;
        let mut v = 0.0;
        while u == 0.0 { u = self.next_f64(); }
        while v == 0.0 { v = self.next_f64(); }
        (-2.0 * u.ln()).sqrt() * (2.0 * PI * v).cos()
    }
}

/// A single debris fragment produced by the collision
#[derive(Debug, Clone)]
pub struct Fragment {
    pub id: usize,
    pub size_m: f64,
    pub mass_kg: f64,
    pub speed_mps: f64,
    pub vel_vec: [f64; 3], // [Radial (X), In-track (Y), Cross-track (Z)]
    pub origin: u8,        // 0 = Target, 1 = Projectile
    pub contour_band: &'static str,
}

/// Simple Breakup Engine
pub struct SimpleBreakupEngine {
    pub target_mass: f64,      // kg
    pub projectile_mass: f64,  // kg
    pub impact_speed: f64,     // m/s
    pub num_fragments: usize,
    pub size_skew: f64,        // Power-law exponent (~1.71)
    pub speed_bias: f64,       // How much faster light fragments fly
}

impl SimpleBreakupEngine {
    pub fn new(target_mass: f64, projectile_mass: f64, impact_speed: f64) -> Self {
        Self {
            target_mass,
            projectile_mass,
            impact_speed,
            num_fragments: 1000,
            size_skew: 1.71,
            speed_bias: 0.60,
        }
    }

    pub fn simulate(&self, seed: u64) -> (Vec<Fragment>, bool, f64, f64) {
        let mut rng = SimpleRng::new(seed);

        // 1. Specific Impact Energy Ep = (M2 * v^2) / (2 * M1) [J/kg]
        let ep_j_per_kg = (self.projectile_mass * self.impact_speed.powi(2)) / (2.0 * self.target_mass);
        let ep_kj_per_kg = ep_j_per_kg / 1000.0;
        let is_catastrophic = ep_j_per_kg >= 40_000.0; // 40 kJ/kg threshold

        // Destroyed mass
        let destroyed_mass = if is_catastrophic {
            self.target_mass + self.projectile_mass
        } else {
            let ejected = self.projectile_mass * (1.0 + (self.impact_speed / 2000.0).powf(1.5));
            (self.target_mass + self.projectile_mass).min(ejected)
        };

        // 2. Generate raw sizes & masses using power-law
        let mut raw_masses = Vec::with_capacity(self.num_fragments);
        let mut sizes = Vec::with_capacity(self.num_fragments);
        let mut total_raw_mass = 0.0;

        for _ in 0..self.num_fragments {
            let u = rng.next_f64();
            // Inverse transform for power law: Lc in [0.01m, 1.5m]
            let lc = 0.01 * (1.0 - u * 0.995).powf(-1.0 / self.size_skew);
            let raw_m = lc.powf(2.3); // Mass scales with physical 3D volume
            sizes.push(lc);
            raw_masses.push(raw_m);
            total_raw_mass += raw_m;
        }

        // 3. Normalize masses to conserve destroyed mass, and compute speed (Delta-v)
        let mut fragments = Vec::with_capacity(self.num_fragments);

        for i in 0..self.num_fragments {
            let mass = (raw_masses[i] / total_raw_mass) * destroyed_mass;
            let lc = sizes[i];

            // Speed based on NASA SBM log-normal distribution:
            // log10(dv) ~ mu(Lc) + sigma * N(0,1)
            let log_lc = lc.log10();
            let mu = 2.02 - 0.58 * log_lc * (self.speed_bias / 0.60);
            let sigma = 0.16;
            let log_dv = mu + sigma * rng.rand_normal();
            let speed = (10.0f64.powf(log_dv) * (self.impact_speed / 10000.0)).clamp(30.0, 3200.0);

            let contour_band = if speed < 250.0 {
                "<250 m/s (Deep Blue)"
            } else if speed < 500.0 {
                "250-500 m/s (Cyan)"
            } else if speed < 1000.0 {
                "500-1000 m/s (Green)"
            } else if speed < 1500.0 {
                "1000-1500 m/s (Yellow)"
            } else if speed < 2000.0 {
                "1500-2000 m/s (Orange)"
            } else {
                ">2000 m/s (Crimson Red)"
            };

            // Isotropic unit vector
            let phi = rng.next_f64() * 2.0 * PI;
            let cos_t = rng.next_f64() * 2.0 - 1.0;
            let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();

            let vx = speed * sin_t * phi.cos();
            let vy = speed * sin_t * phi.sin();
            let vz = speed * cos_t;

            let origin = if rng.next_f64() < (self.target_mass / (self.target_mass + self.projectile_mass)) { 0 } else { 1 };

            fragments.push(Fragment {
                id: i,
                size_m: lc,
                mass_kg: mass,
                speed_mps: speed,
                vel_vec: [vx, vy, vz],
                origin,
                contour_band,
            });
        }

        (fragments, is_catastrophic, ep_kj_per_kg, destroyed_mass)
    }
}

fn main() {
    println!("============================================================");
    println!(" NASA Standard Breakup Model - Simple Engine CLI");
    println!("============================================================");

    let target_m = 1000.0;    // kg
    let proj_m = 100.0;       // kg
    let impact_v = 10_000.0;  // 10 km/s (10,000 m/s)

    let mut engine = SimpleBreakupEngine::new(target_m, proj_m, impact_v);
    engine.num_fragments = 1500;

    let (fragments, is_catastrophic, ep_kj, destroyed_m) = engine.simulate(42);

    println!("Target Mass:       {:.0} kg", target_m);
    println!("Projectile Mass:   {:.0} kg", proj_m);
    println!("Impact Velocity:   {:.1} km/s", impact_v / 1000.0);
    println!("Specific Energy:   {:.1} kJ/kg", ep_kj);
    println!("Collision Outcome: {}", if is_catastrophic { "CATASTROPHIC (Total breakup)" } else { "PARTIAL (Crater/Remnant)" });
    println!("Destroyed Mass:    {:.1} kg", destroyed_m);
    println!("Total Fragments:   {}", fragments.len());
    println!("------------------------------------------------------------");

    // Print top 3 heaviest fragments
    let mut sorted_by_mass = fragments.clone();
    sorted_by_mass.sort_by(|a, b| b.mass_kg.partial_cmp(&a.mass_kg).unwrap());

    println!("TOP 3 HEAVIEST FRAGMENTS (Core chunks, stay near center):");
    for f in &sorted_by_mass[..3] {
        println!("  #{} -> Size: {:.2} m, Mass: {:.1} kg, Speed: {:.0} m/s, Band: {}", f.id, f.size_m, f.mass_kg, f.speed_mps, f.contour_band);
    }

    println!("\nTOP 3 FASTEST FRAGMENTS (Light shards, outer expanding bubble):");
    let mut sorted_by_speed = fragments.clone();
    sorted_by_speed.sort_by(|a, b| b.speed_mps.partial_cmp(&a.speed_mps).unwrap());
    for f in &sorted_by_speed[..3] {
        println!("  #{} -> Size: {:.1} cm, Mass: {:.3} kg, Speed: {:.0} m/s, Band: {}", f.id, f.size_m * 100.0, f.mass_kg, f.speed_mps, f.contour_band);
    }

    // Export to JSON for inspection or feeding to visualizers
    let mut json_file = File::create("fragments_output.json").expect("Failed to create file");
    write!(json_file, "[\n").unwrap();
    for (i, f) in fragments.iter().enumerate() {
        let comma = if i + 1 < fragments.len() { "," } else { "" };
        write!(
            json_file,
            "  {{\"id\":{},\"size\":{:.4},\"mass\":{:.4},\"speed\":{:.1},\"contour\":\"{}\",\"vx\":{:.1},\"vy\":{:.1},\"vz\":{:.1}}}{}\n",
            f.id, f.size_m, f.mass_kg, f.speed_mps, f.contour_band, f.vel_vec[0], f.vel_vec[1], f.vel_vec[2], comma
        ).unwrap();
    }
    write!(json_file, "]\n").unwrap();
    println!("\n[OK] Exported {} fragments to fragments_output.json", fragments.len());
    println!("============================================================");
}
