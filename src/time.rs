use chrono::{Datelike, Local, NaiveDate, Timelike};
use ekreta::LDateTime;

/// `DateTime` methods needed for `RsFilc`
pub trait MyDate: Datelike + std::fmt::Display {
    /// Practical date format.
    fn pretty(&self) -> String;
    /// Get hungarian month.
    fn hun_month<'a>(&self) -> &'a str {
        match self.month() {
            1 => "jan.",
            2 => "feb.",
            3 => "már.",
            4 => "ápr.",
            5 => "máj.",
            6 => "jún.",
            7 => "júl.",
            8 => "aug.",
            9 => "szep.",
            10 => "okt.",
            11 => "nov.",
            12 => "dec.",
            _ => unreachable!("invalid month"),
        }
    }
    /// Get hungarian day of week.
    fn hun_day_of_week<'a>(&self) -> &'a str {
        match self.weekday() {
            chrono::Weekday::Mon => "hétfő",
            chrono::Weekday::Tue => "kedd",
            chrono::Weekday::Wed => "szerda",
            chrono::Weekday::Thu => "csütörtök",
            chrono::Weekday::Fri => "péntek",
            chrono::Weekday::Sat => "szombat",
            chrono::Weekday::Sun => "vasárnap",
        }
    }
    fn day_diff(&self, other: &Self) -> Option<String> {
        let day_diff = self.num_days_from_ce() - other.num_days_from_ce();
        match day_diff {
            -2 => Some("tegnapelőtt".into()),
            -1 => Some("tegnap".into()),
            0 => Some("ma".into()),
            1 => Some("holnap".into()),
            2 => Some("holnapután".into()),
            _ => None,
        }
    }
}

impl MyDate for LDateTime {
    fn pretty(&self) -> String {
        if self.year() != Local::now().year() {
            format!("{}", self.format("%Y.%m.%d"))
        } else if let Some(day_diff) = self.day_diff(&Local::now()) {
            format!(
                "{day_diff} {}",
                self.format(if self.hour() == 0 && self.minute() == 0 {
                    "%d."
                } else {
                    "%d. %H:%M"
                })
            )
        } else {
            format!(
                "{} {}",
                self.hun_month(),
                self.format(if self.hour() == 0 && self.minute() == 0 {
                    "%d."
                } else {
                    "%d. %H:%M"
                })
            )
        }
    }
}

impl MyDate for NaiveDate {
    fn pretty(&self) -> String {
        if self.year() != Local::now().year() {
            format!("{self}")
        } else if let Some(day_diff) = self.day_diff(&Local::now().date_naive()) {
            format!("{day_diff}")
        } else {
            format!("{} {:02}", self.hun_month(), self.day())
        }
    }
}
