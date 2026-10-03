//! Modular subcommands for sbm_cli.

pub mod avoidance;
pub mod breakup;
pub mod rpo;
pub mod transfer;

pub use avoidance::{run_avoidance_eval_cli, run_avoidance_train_cli};
pub use breakup::run_breakup_cli;
pub use rpo::run_rpo_cli;
pub use transfer::run_transfer_cli;
