//! Границы дня и недели.
//!
//! Единственное место, где решается, к какому дню относится событие.

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayRules {
    /// Во сколько начинается новый день (0–23).
    pub day_start_hour: u32,
    /// Неделя начинается в понедельник, иначе в воскресенье.
    pub week_starts_monday: bool,
}

impl Default for DayRules {
    fn default() -> Self {
        Self {
            day_start_hour: 0,
            week_starts_monday: true,
        }
    }
}

impl DayRules {
    pub fn new(day_start_hour: u32, week_starts_monday: bool) -> Self {
        Self {
            day_start_hour: day_start_hour.min(23),
            week_starts_monday,
        }
    }

    /// Локальная дата события с учётом начала дня.
    pub fn local_date(&self, ts: DateTime<FixedOffset>) -> NaiveDate {
        let shifted = ts - Duration::hours(self.day_start_hour as i64);
        shifted.date_naive()
    }

    /// Первый день недели, в которую попадает дата.
    pub fn week_start(&self, date: NaiveDate) -> NaiveDate {
        let weekday = date.weekday().num_days_from_monday();
        let offset = if self.week_starts_monday {
            weekday
        } else {
            (weekday + 1) % 7
        };
        date - Duration::days(offset as i64)
    }

    pub fn week_end(&self, date: NaiveDate) -> NaiveDate {
        self.week_start(date) + Duration::days(6)
    }

    pub fn is_same_week(&self, a: NaiveDate, b: NaiveDate) -> bool {
        self.week_start(a) == self.week_start(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(4 * 3600)
            .unwrap()
            .with_ymd_and_hms(y, m, d, h, min, 0)
            .unwrap()
    }

    #[test]
    fn day_starts_at_four_in_the_morning() {
        let rules = DayRules::new(4, true);
        let late_night = rules.local_date(ts(2024, 3, 10, 3, 30));
        assert_eq!(late_night, NaiveDate::from_ymd_opt(2024, 3, 9).unwrap());
        let morning = rules.local_date(ts(2024, 3, 10, 4, 0));
        assert_eq!(morning, NaiveDate::from_ymd_opt(2024, 3, 10).unwrap());
    }

    #[test]
    fn midnight_by_default() {
        let rules = DayRules::default();
        assert_eq!(
            rules.local_date(ts(2024, 3, 10, 0, 0)),
            NaiveDate::from_ymd_opt(2024, 3, 10).unwrap()
        );
    }

    #[test]
    fn week_can_start_on_sunday() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap(); // воскресенье
        let monday = DayRules::new(0, true);
        let sunday = DayRules::new(0, false);
        assert_eq!(
            monday.week_start(date),
            NaiveDate::from_ymd_opt(2024, 3, 4).unwrap()
        );
        assert_eq!(sunday.week_start(date), date);
    }

    #[test]
    fn weeks_do_not_overlap() {
        let rules = DayRules::default();
        let mut date = NaiveDate::from_ymd_opt(2023, 12, 20).unwrap();
        let mut previous: Option<NaiveDate> = None;
        for _ in 0..40 {
            let start = rules.week_start(date);
            if let Some(prev) = previous {
                assert!(start > prev);
                assert_eq!(prev + Duration::days(7), start);
            }
            previous = Some(start);
            date += Duration::days(7);
        }
    }

    #[test]
    fn day_start_hour_is_clamped() {
        assert_eq!(DayRules::new(99, true).day_start_hour, 23);
        assert_eq!(DayRules::new(0, false).day_start_hour, 0);
    }

    #[test]
    fn week_end_is_six_days_after_start_and_matches_same_week() {
        let rules = DayRules::new(4, true);
        let wednesday = NaiveDate::from_ymd_opt(2024, 3, 6).unwrap();
        let start = rules.week_start(wednesday);
        assert_eq!(rules.week_end(wednesday), start + Duration::days(6));
        assert!(rules.is_same_week(start, rules.week_end(wednesday)));
        assert!(!rules.is_same_week(start, start + Duration::days(7)));
    }

    #[test]
    fn local_date_crosses_month_and_year_boundaries() {
        let rules = DayRules::new(4, true);
        let new_year_night = rules.local_date(ts(2025, 1, 1, 2, 0));
        assert_eq!(
            new_year_night,
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()
        );
        let after_start = rules.local_date(ts(2025, 1, 1, 4, 0));
        assert_eq!(after_start, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    }
}
