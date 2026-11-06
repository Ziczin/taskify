//! Кастомные метрики и эффекты от выполнений.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricOp {
    Max,
    Set,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricEffect {
    /// `metric = 1` — прибавить единицу.
    Increment(i64),
    /// `metric = { op = "max", value = 5 }`.
    Full { op: MetricOp, value: i64 },
}

impl MetricEffect {
    pub fn apply(self, current: i64) -> i64 {
        match self {
            MetricEffect::Increment(delta) => current.saturating_add(delta),
            MetricEffect::Full { op, value } => match op {
                MetricOp::Add => current.saturating_add(value),
                MetricOp::Max => current.max(value),
                MetricOp::Set => value,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn increment_adds() {
        assert_eq!(MetricEffect::Increment(1).apply(4), 5);
        assert_eq!(MetricEffect::Increment(-2).apply(4), 2);
    }

    #[test]
    fn max_keeps_bigger() {
        let e = MetricEffect::Full {
            op: MetricOp::Max,
            value: 10,
        };
        assert_eq!(e.apply(3), 10);
        assert_eq!(e.apply(30), 30);
    }

    #[test]
    fn set_replaces() {
        let e = MetricEffect::Full {
            op: MetricOp::Set,
            value: 7,
        };
        assert_eq!(e.apply(100), 7);
    }

    #[test]
    fn toml_forms_parse() {
        #[derive(Deserialize)]
        struct Holder {
            a: MetricEffect,
            b: MetricEffect,
        }
        let holder: Holder = toml::from_str("a = 1\nb = { op = \"max\", value = 5 }\n").unwrap();
        assert_eq!(holder.a, MetricEffect::Increment(1));
        assert_eq!(
            holder.b,
            MetricEffect::Full {
                op: MetricOp::Max,
                value: 5
            }
        );
    }
}
