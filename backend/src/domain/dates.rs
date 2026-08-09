use chrono::{Datelike, NaiveDate};

/// Add months to a date, clamping to the last day of the target month.
/// Panics in debug mode if months is not positive.
pub fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    debug_assert!(
        months > 0,
        "add_months requires a positive month count, got {months}"
    );

    let total_months = date.month0() as i32 + months;
    let target_year = date.year() + total_months / 12;
    let target_month = (total_months % 12) as u32 + 1;

    // Try the same day, then clamp to last day of month
    NaiveDate::from_ymd_opt(target_year, target_month, date.day())
        .or_else(|| {
            // Last day of target month
            let next_month = if target_month == 12 {
                NaiveDate::from_ymd_opt(target_year + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(target_year, target_month + 1, 1)
            };
            next_month.map(|d| d - chrono::Duration::days(1))
        })
        .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_months_basic() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert_eq!(
            add_months(date, 12),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
    }

    #[test]
    fn test_add_months_year_boundary() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        assert_eq!(
            add_months(date, 12),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
        );
    }

    #[test]
    fn test_add_months_leap_year() {
        let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        // 2025 is not a leap year, so Feb 29 clamps to Feb 28
        assert_eq!(
            add_months(date, 12),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "add_months requires a positive month count")]
    fn test_add_months_rejects_zero() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        add_months(date, 0);
    }

    #[test]
    #[should_panic(expected = "add_months requires a positive month count")]
    fn test_add_months_rejects_negative() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        add_months(date, -1);
    }
}
