//! Пересчёт состояния и статистики из журнала.

use std::collections::BTreeSet;

use crate::aggregates::Aggregates;
use crate::apply::{apply, Rules};
use crate::event::Event;
use crate::state::State;

#[derive(Debug, Default)]
pub struct ReplayResult {
    pub state: State,
    pub aggregates: Aggregates,
    pub warnings: Vec<String>,
    pub events: usize,
}

pub fn rebuild(events: &[Event], rules: &Rules) -> ReplayResult {
    let mut sorted: Vec<&Event> = events.iter().collect();
    sorted.sort_by_key(|event| event.seq);

    let mut state = State::default();
    let mut aggregates = Aggregates::default();
    let mut warnings = Vec::new();
    let mut seen_ids: BTreeSet<&str> = BTreeSet::new();
    let mut expected_seq = 1u64;

    for event in &sorted {
        if !seen_ids.insert(event.id.as_str()) {
            warnings.push(format!("повторяющийся id события: {}", event.id));
        }
        if event.seq != expected_seq {
            if event.seq > expected_seq {
                warnings.push(format!(
                    "пропуск в нумерации: ожидалось {}, пришло {}",
                    expected_seq, event.seq
                ));
            } else {
                warnings.push(format!("событие с обратным порядком: {}", event.seq));
            }
        }
        expected_seq = event.seq.saturating_add(1);
        apply(&mut state, &mut aggregates, event, rules);
    }

    ReplayResult {
        state,
        aggregates,
        warnings,
        events: sorted.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventKind, HabitSkipped};
    use chrono::TimeZone;

    fn skip(seq: u64) -> Event {
        let ts = chrono::FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 3, 10, 9, 0, 0)
            .unwrap();
        Event {
            schema: 1,
            seq,
            id: format!("e{seq}"),
            ts,
            kind: EventKind::HabitSkipped(HabitSkipped {
                task_id: "water".into(),
                local_date: ts.date_naive(),
            }),
        }
    }

    #[test]
    fn rebuild_is_deterministic_and_reports_gaps() {
        let events = vec![skip(1), skip(3)];
        let rules = Rules::default();
        let result = rebuild(&events, &rules);
        assert_eq!(result.events, 2);
        assert_eq!(result.state.last_seq, 3);
        assert!(result.warnings.iter().any(|w| w.contains("пропуск")));

        let again = rebuild(&events, &rules);
        assert_eq!(
            again.aggregates.totals.skips,
            result.aggregates.totals.skips
        );
    }
}
