//! Кривая уровней без потолка.
//!
//! Первый переход стоит 50 опыта, и каждый следующий — на 10 больше:
//!
//! ```text
//! уровень 2   50
//! уровень 3   50 + 60 = 110
//! уровень 4   50 + 60 + 70 = 180
//! ...
//! ```
//!
//! Суммарный опыт для уровня `L` (`n = L - 1`):
//!
//! ```text
//! 50n + 10 * n(n - 1) / 2 = 5n² + 45n
//! ```
//!
//! Кривая считается в `u128` и насыщается на `u64::MAX`, поэтому потолка
//! уровней нет: максимальный уровень — около 1,9 млрд, дальше опыт уже не
//! помещается в `u64`.

/// Суммарный опыт, необходимый для уровня.
pub fn xp_to_reach(level: u32) -> u64 {
    if level <= 1 {
        return 0;
    }
    let n = (level - 1) as u128;
    let value = 5 * n * n + 45 * n;
    if value >= u64::MAX as u128 {
        u64::MAX
    } else {
        value as u64
    }
}

/// Наибольший уровень, порог которого ещё помещается в `u64`.
pub fn max_level() -> u32 {
    let mut low = 1u32;
    let mut high = u32::MAX;
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if xp_to_reach(mid) < u64::MAX {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

/// Уровень по суммарному опыту.
pub fn level_for(xp: i64) -> u32 {
    if xp <= 0 {
        return 1;
    }
    let xp = xp as u64;
    let mut low = 1u32;
    let mut high = max_level().max(1);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if xp_to_reach(mid) <= xp {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

/// Опыт внутри текущего уровня и сколько нужно до следующего.
pub fn level_progress(xp: i64) -> (u64, u64) {
    let level = level_for(xp);
    let base = xp_to_reach(level);
    let next = xp_to_reach(level.saturating_add(1));
    let inside = (xp.max(0) as u64).saturating_sub(base);
    let span = next.saturating_sub(base);
    (inside.min(span), span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_start_at_fifty_and_grow_by_ten() {
        assert_eq!(xp_to_reach(1), 0);
        assert_eq!(xp_to_reach(2), 50);
        assert_eq!(xp_to_reach(3), 110);
        assert_eq!(xp_to_reach(4), 180);
        assert_eq!(xp_to_reach(5), 260);
        assert_eq!(xp_to_reach(10), 810);
    }

    #[test]
    fn every_level_costs_ten_more_than_the_previous() {
        let mut previous = xp_to_reach(2);
        let mut span = previous;
        for level in 3..10_000 {
            let current = xp_to_reach(level);
            let next_span = current - previous;
            assert_eq!(
                next_span,
                span + 10,
                "уровень {level} требует {next_span} вместо {}",
                span + 10
            );
            span = next_span;
            previous = current;
        }
    }

    #[test]
    fn curve_is_monotonic_up_to_saturation() {
        let mut previous = 0;
        for level in 1..2_000 {
            let value = xp_to_reach(level);
            assert!(value >= previous, "уровень {level} дешевле предыдущего");
            previous = value;
        }
    }

    #[test]
    fn level_lookup_matches_thresholds() {
        assert_eq!(level_for(-5), 1);
        assert_eq!(level_for(0), 1);
        assert_eq!(level_for(49), 1);
        assert_eq!(level_for(50), 2);
        assert_eq!(level_for(109), 2);
        assert_eq!(level_for(110), 3);
        assert_eq!(level_for(5000), 28);
        assert_eq!(level_for(1_000_000), 443);
    }

    #[test]
    fn no_level_cap_and_saturation() {
        assert!(max_level() > 1_000_000);
        assert_eq!(xp_to_reach(u32::MAX), u64::MAX);
        let top = level_for(i64::MAX);
        assert!(
            top > 1_000_000,
            "уровень по максимуму i64 слишком мал: {top}"
        );
        assert!(top < max_level());
    }

    #[test]
    fn progress_stays_inside_level() {
        let (inside, span) = level_progress(10);
        assert_eq!(inside, 10);
        assert_eq!(span, 50);
        let (inside, span) = level_progress(105);
        assert_eq!(inside, 55);
        assert_eq!(span, 60);
        assert!(inside < span);
    }
}
