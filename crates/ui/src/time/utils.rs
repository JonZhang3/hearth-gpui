use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[cfg(test)]
trait NaiveDateExt {
    fn days_in_month(&self) -> i32;
    fn is_leap_year(&self) -> bool;
}

#[cfg(test)]
impl NaiveDateExt for chrono::NaiveDate {
    fn days_in_month(&self) -> i32 {
        let month = self.month();
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.is_leap_year() {
                    29
                } else {
                    28
                }
            }
            _ => panic!("Invalid month: {}", month),
        }
    }

    fn is_leap_year(&self) -> bool {
        let year = self.year();
        return year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    }
}

pub(crate) fn days_in_month(year: i32, month: i32, week_starts_on: Weekday) -> Vec<Vec<NaiveDate>> {
    // Normalize arbitrary positive and negative month offsets before constructing the date.
    let month_index = year * 12 + month - 1;
    let normalized_year = month_index.div_euclid(12);
    let normalized_month = month_index.rem_euclid(12) as u32 + 1;
    let first = NaiveDate::from_ymd_opt(normalized_year, normalized_month, 1)
        .expect("normalized calendar month must be valid");

    let first_weekday = first.weekday().num_days_from_monday() as i64;
    let week_start = week_starts_on.num_days_from_monday() as i64;
    let leading_days = (first_weekday - week_start).rem_euclid(7);
    let grid_start = first - Duration::days(leading_days);

    // Six rows keep Calendar and DatePicker geometry stable for every month.
    (0..6)
        .map(|week| {
            (0..7)
                .map(|day| grid_start + Duration::days((week * 7 + day) as i64))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate, Weekday};

    use super::{NaiveDateExt, days_in_month};

    #[test]
    fn test_days_in_month() {
        assert_eq!(
            NaiveDate::from_ymd_opt(2024, 2, 1).unwrap().days_in_month(),
            29
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2023, 2, 1).unwrap().days_in_month(),
            28
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().days_in_month(),
            31
        );
        assert_eq!(
            NaiveDate::from_ymd_opt(2023, 4, 1).unwrap().days_in_month(),
            30
        );
    }

    #[test]
    fn test_days() {
        #[track_caller]
        fn assert_case(date: NaiveDate, expected: Vec<&str>) {
            let out = days_in_month(date.year(), date.month() as i32, Weekday::Sun)
                .iter()
                .map(|week| {
                    week.iter()
                        .map(|d| {
                            if d.year() == date.year() && d.month() == date.month() {
                                format!("{:2}", d.day())
                            } else if d.year() == date.year() {
                                format!("{}-{}", d.month(), d.day())
                            } else {
                                format!("{}-{}-{}", d.year(), d.month(), d.day())
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .collect::<Vec<_>>();

            assert_eq!(out, expected);
        }

        assert_case(
            NaiveDate::from_ymd_opt(2024, 8, 1).unwrap(),
            vec![
                "7-28|7-29|7-30|7-31| 1| 2| 3",
                " 4| 5| 6| 7| 8| 9|10",
                "11|12|13|14|15|16|17",
                "18|19|20|21|22|23|24",
                "25|26|27|28|29|30|31",
                "9-1|9-2|9-3|9-4|9-5|9-6|9-7",
            ],
        );
        assert_case(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            vec![
                "2024-12-29|2024-12-30|2024-12-31| 1| 2| 3| 4",
                " 5| 6| 7| 8| 9|10|11",
                "12|13|14|15|16|17|18",
                "19|20|21|22|23|24|25",
                "26|27|28|29|30|31|2-1",
                "2-2|2-3|2-4|2-5|2-6|2-7|2-8",
            ],
        );

        assert_case(
            NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
            vec![
                "1-28|1-29|1-30|1-31| 1| 2| 3",
                " 4| 5| 6| 7| 8| 9|10",
                "11|12|13|14|15|16|17",
                "18|19|20|21|22|23|24",
                "25|26|27|28|29|3-1|3-2",
                "3-3|3-4|3-5|3-6|3-7|3-8|3-9",
            ],
        );
        assert_case(
            NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
            vec![
                "1-29|1-30|1-31| 1| 2| 3| 4",
                " 5| 6| 7| 8| 9|10|11",
                "12|13|14|15|16|17|18",
                "19|20|21|22|23|24|25",
                "26|27|28|3-1|3-2|3-3|3-4",
                "3-5|3-6|3-7|3-8|3-9|3-10|3-11",
            ],
        );
    }

    #[test]
    fn supports_monday_week_start_and_arbitrary_month_offsets() {
        let weeks = days_in_month(2024, 14, Weekday::Mon);
        assert_eq!(weeks[0][0], NaiveDate::from_ymd_opt(2025, 1, 27).unwrap());
        assert_eq!(weeks[5][6], NaiveDate::from_ymd_opt(2025, 3, 9).unwrap());
    }
}
