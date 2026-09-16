use chrono::{Datelike, NaiveDate};

use crate::{AppError, AppResult, models::DerivedEmploymentDuration};

pub fn employment_duration(
    start_date: NaiveDate,
    as_of: NaiveDate,
) -> AppResult<DerivedEmploymentDuration> {
    if start_date > as_of {
        return Err(AppError::Validation(
            "employment start date cannot be in the future".to_owned(),
        ));
    }

    let mut total_months =
        (as_of.year() - start_date.year()) * 12 + as_of.month() as i32 - start_date.month() as i32;
    if as_of.day() < start_date.day() {
        total_months -= 1;
    }

    let years = total_months / 12;
    let months = (total_months % 12) as u32;
    let display = match (years, months) {
        (0, 0) => "Less than one month".to_owned(),
        (0, month_count) => pluralize(month_count, "month"),
        (year_count, 0) => pluralize(year_count as u32, "year"),
        (year_count, month_count) => format!(
            "{} {}",
            pluralize(year_count as u32, "year"),
            pluralize(month_count, "month")
        ),
    };

    Ok(DerivedEmploymentDuration {
        years,
        months,
        display,
        start_date: start_date.to_string(),
        as_of: as_of.to_string(),
    })
}

fn pluralize(value: u32, noun: &str) -> String {
    let suffix = if value == 1 { "" } else { "s" };
    format!("{value} {noun}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_complete_months() {
        let duration = employment_duration(
            NaiveDate::from_ymd_opt(2025, 3, 20).expect("date"),
            NaiveDate::from_ymd_opt(2026, 9, 15).expect("date"),
        )
        .expect("duration");

        assert_eq!(duration.years, 1);
        assert_eq!(duration.months, 5);
        assert_eq!(duration.display, "1 year 5 months");
    }

    #[test]
    fn rejects_future_start_dates() {
        let result = employment_duration(
            NaiveDate::from_ymd_opt(2027, 1, 1).expect("date"),
            NaiveDate::from_ymd_opt(2026, 1, 1).expect("date"),
        );

        assert!(matches!(result, Err(AppError::Validation(_))));
    }
}
