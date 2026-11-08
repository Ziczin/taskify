//! Домен Taskify.

pub mod group;
pub mod level;
pub mod metrics;
pub mod reward;
pub mod task;
pub mod time;

pub use group::{GroupError, RandomGroup};
pub use level::{level_for, level_progress, max_level, xp_to_reach};
pub use metrics::{MetricEffect, MetricOp};
pub use reward::{award, Milestone, MilestoneHit, Reward};
pub use task::{tr, HintCost, Hints, Kind, Locales, Schedule, Subtask, Task, TaskSnapshot};

/// Номер сборки крейта, виден в «О программе».
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION_MAJOR")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_not_empty() {
        assert!(!super::version().is_empty());
    }
}
