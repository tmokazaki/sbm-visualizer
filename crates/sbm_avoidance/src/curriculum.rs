//! Curriculum learning schedule for training stability across progressively dense debris fields.
//!
//! Grounded in Table 1 and Section IV-B1 of Luna et al. (2026):
//! - **Stage 1 (Basic avoidance)**: $0 \rightarrow 250,000$ steps ($p_{\text{coll}} = 0.4, r_{\text{deb}} = 25\text{ m}$)
//! - **Stage 2 (Intermediate)**: $250,000 \rightarrow 600,000$ steps ($p_{\text{coll}} = 0.6, r_{\text{deb}} = 50\text{ m}$)
//! - **Stage 3 (Advanced)**: $600,000 \rightarrow 1,000,000$ steps ($p_{\text{coll}} = 1.0, r_{\text{deb}} = 100\text{ m}$)

use serde::{Deserialize, Serialize};

/// Curriculum stage identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurriculumStageName {
    BasicAvoidance,
    Intermediate,
    Advanced,
}

/// Parameters governing a curriculum stage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CurriculumStage {
    pub name: CurriculumStageName,
    pub step_start: usize,
    pub step_end: usize,
    pub collision_probability: f64,
    pub debris_radius_m: f64,
}

/// Curriculum schedule coordinator managing training scenario difficulty over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurriculumScheduler {
    pub stages: Vec<CurriculumStage>,
}

impl Default for CurriculumScheduler {
    fn default() -> Self {
        Self {
            stages: vec![
                CurriculumStage {
                    name: CurriculumStageName::BasicAvoidance,
                    step_start: 0,
                    step_end: 250_000,
                    collision_probability: 0.4,
                    debris_radius_m: 25.0,
                },
                CurriculumStage {
                    name: CurriculumStageName::Intermediate,
                    step_start: 250_000,
                    step_end: 600_000,
                    collision_probability: 0.6,
                    debris_radius_m: 50.0,
                },
                CurriculumStage {
                    name: CurriculumStageName::Advanced,
                    step_start: 600_000,
                    step_end: usize::MAX,
                    collision_probability: 1.0,
                    debris_radius_m: 100.0,
                },
            ],
        }
    }
}

impl CurriculumScheduler {
    /// Returns the active curriculum stage for a given global training step count.
    pub fn get_stage(&self, global_step: usize) -> CurriculumStage {
        for stage in &self.stages {
            if global_step >= stage.step_start && global_step < stage.step_end {
                return *stage;
            }
        }
        *self.stages.last().expect("Curriculum must have at least one stage")
    }
}
