//! Кривая уровней без потолка.
//!
//! Порог суммарного опыта задан четырьмя участками ускорения; `x` — номер
//! порога, то есть `level - 1`, поэтому первый уровень начинается с нуля:
//!
//! ```text
//! x <= 10          10x
//! 10 < x <= 100    ceil(x^2 / 2 + max(0, 60 - x))
//! 100 < x <= 1000  floor(9x^3 / (8x + 1000))
//! x > 1000         floor((x/10)^log10(x) * (x/1000)^-p)
//! ```
//!
//! Четвёртый участок домножен на плавную поправку `(x/1000)^-p`: при `x = 1000`
//! она равна единице и не рвёт стык, а к эталонному уровню 77 777 гасит значение
//! до 7 777 777 777 777 777 777 опыта. Показатель `p` посчитан из этого условия
//! и зафиксирован константой.
//!
//! Потолка уровней нет: кривая считается в `f64` и насыщается на `u64::MAX`.
//! Максимальный достижимый уровень — около 85 843, дальше опыт уже не помещается
//! в `u64`.

/// Уровень, до которого кривая откалибрована.
pub const REFERENCE_LEVEL: u32 = 77_777;

/// Целевой опыт эталонного уровня.
pub const REFERENCE_XP: u64 = 7_777_777_777_777_777_777;

/// Показатель поправки четвёртого участка.
///
/// `p = ln(base(REFERENCE_LEVEL - 1) / REFERENCE_XP) / ln((REFERENCE_LEVEL - 1) / 1000)`.
const CORRECTION: f64 = 0.073_337_339_098_789_88;

/// Суммарный опыт, необходимый для уровня.
pub fn xp_to_reach(level: u32) -> u64 {
    if level <= 1 {
        return 0;
    }
    let n = (level - 1) as u128;

    if n <= 10 {
        return (10 * n) as u64;
    }
    if n <= 100 {
        let square = (n * n).div_ceil(2);
        let tail = 60u128.saturating_sub(n);
        return (square + tail) as u64;
    }
    if n <= 1000 {
        let numerator = 9 * n * n * n;
        let denominator = 8 * n + 1000;
        return (numerator / denominator) as u64;
    }

    let x = n as f64;
    let base = (x / 10.0).powf(x.log10());
    let value = base * (x / 1000.0).powf(-CORRECTION);
    if !value.is_finite() || value >= u64::MAX as f64 {
        return u64::MAX;
    }
    value as u64
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
    fn first_ten_levels_are_linear() {
        assert_eq!(xp_to_reach(1), 0);
        assert_eq!(xp_to_reach(2), 10);
        assert_eq!(xp_to_reach(10), 90);
        assert_eq!(xp_to_reach(11), 100);
    }

    #[test]
    fn segments_join_without_jumps() {
        assert_eq!(xp_to_reach(101), 5000);
        assert_eq!(xp_to_reach(1001), 1_000_000);
        assert!(xp_to_reach(100) < xp_to_reach(101));
        assert!(xp_to_reach(1000) < xp_to_reach(1001));
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
    fn reference_level_costs_about_seven_point_seven_quintillion() {
        let value = xp_to_reach(REFERENCE_LEVEL) as f64;
        let target = REFERENCE_XP as f64;
        let error = (value - target).abs() / target;
        assert!(error < 0.000_001, "ошибка калибровки {error}");
    }

    #[test]
    fn level_lookup_matches_thresholds() {
        assert_eq!(level_for(-5), 1);
        assert_eq!(level_for(0), 1);
        assert_eq!(level_for(10), 2);
        assert_eq!(level_for(99), 10);
        assert_eq!(level_for(100), 11);
        assert_eq!(level_for(5000), 101);
        assert_eq!(level_for(1_000_000), 1001);
        assert_eq!(level_for(REFERENCE_XP as i64), REFERENCE_LEVEL - 1);
    }

    #[test]
    fn no_level_cap_and_saturation() {
        assert!(max_level() > 70_000);
        assert_eq!(xp_to_reach(u32::MAX), u64::MAX);
        let top = level_for(i64::MAX);
        assert!(top > 60_000, "уровень по максимуму i64 слишком мал: {top}");
        assert!(top < max_level());
    }

    #[test]
    fn progress_stays_inside_level() {
        let (inside, span) = level_progress(10);
        assert_eq!(inside, 0);
        assert_eq!(span, 10);
        let (inside, span) = level_progress(105);
        assert!(inside < span);
    }
}
