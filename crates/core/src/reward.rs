//! Награды и рубежи.
//!
//! Награда = база + сумма независимых бонусов за каждый достигнутый рубеж.
//! Отрицательные значения форматом допустимы, но базовый пак их не использует.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Reward {
    #[serde(default)]
    pub xp: i64,
    #[serde(default)]
    pub coins: i64,
}

impl Reward {
    pub fn new(xp: i64, coins: i64) -> Self {
        Self { xp, coins }
    }

    pub fn is_zero(&self) -> bool {
        self.xp == 0 && self.coins == 0
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Reward) -> Reward {
        Reward {
            xp: self.xp.saturating_add(other.xp),
            coins: self.coins.saturating_add(other.coins),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub every: u32,
    #[serde(default = "one")]
    pub xp_mult: f64,
    #[serde(default = "one")]
    pub coins_mult: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MilestoneHit {
    pub every: u32,
    pub xp_mult: f64,
    pub coins_mult: f64,
}

/// Награда за `k`-е выполнение.
pub fn award(base: Reward, milestones: &[Milestone], k: u64) -> (Reward, Vec<MilestoneHit>) {
    let mut xp = base.xp as f64;
    let mut coins = base.coins as f64;
    let mut hits = Vec::new();

    if k > 0 {
        for m in milestones {
            if m.every == 0 {
                continue;
            }
            if k % m.every as u64 == 0 {
                xp += base.xp as f64 * m.xp_mult;
                coins += base.coins as f64 * m.coins_mult;
                hits.push(MilestoneHit {
                    every: m.every,
                    xp_mult: m.xp_mult,
                    coins_mult: m.coins_mult,
                });
            }
        }
    }

    (
        Reward {
            xp: xp.round() as i64,
            coins: coins.round() as i64,
        },
        hits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn milestones() -> Vec<Milestone> {
        vec![
            Milestone {
                every: 7,
                xp_mult: 2.0,
                coins_mult: 2.0,
            },
            Milestone {
                every: 100,
                xp_mult: 10.0,
                coins_mult: 10.0,
            },
        ]
    }

    #[test]
    fn seven_hundredth_completion_hits_both_milestones() {
        let (reward, hits) = award(Reward::new(10, 2), &milestones(), 700);
        assert_eq!(reward.xp, 130);
        assert_eq!(reward.coins, 26);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn first_completion_has_no_milestones() {
        let (reward, hits) = award(Reward::new(10, 2), &milestones(), 1);
        assert_eq!(reward, Reward::new(10, 2));
        assert!(hits.is_empty());
    }

    #[test]
    fn seventh_completion_doubles_base() {
        let (reward, hits) = award(Reward::new(10, 2), &milestones(), 7);
        assert_eq!(reward, Reward::new(30, 6));
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn negative_base_is_allowed() {
        let (reward, _) = award(Reward::new(-5, -1), &[], 3);
        assert_eq!(reward, Reward::new(-5, -1));
    }

    #[test]
    fn zero_milestone_every_is_ignored() {
        let m = vec![Milestone {
            every: 0,
            xp_mult: 5.0,
            coins_mult: 5.0,
        }];
        let (reward, hits) = award(Reward::new(10, 2), &m, 10);
        assert_eq!(reward, Reward::new(10, 2));
        assert!(hits.is_empty());
    }
}
