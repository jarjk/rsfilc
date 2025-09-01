//! lessons the student has

use crate::{time::MyDate, user::User};
use chrono::{Datelike, Local, NaiveDate, TimeDelta};
use ekreta::{AnnouncedTest, LDateTime, Lesson, Res};
use log::*;
use yansi::Paint;

pub fn handle(day: Option<NaiveDate>, user: &User, current: bool, json: bool) -> Res<()> {
    debug!("requested day: {day:?}");
    let day = day.unwrap_or_else(|| default_day(user));
    debug!("showing day: {day}");
    let lessons_of_week = user.get_timetable(day, true)?;
    let lessons = user.get_timetable(day, false)?;
    if lessons.is_empty() {
        if json {
            println!("null");
        } else {
            println!("{day} ({}) nincs rögzített órád, juhé!", day.weekday());
        }
        return Ok(());
    }
    if current {
        if let Some(nxt) = next_lesson(&lessons_of_week) {
            if json {
                let data = serde_json::to_string(&(mins_till(nxt.kezdet_idopont), nxt))?;
                println!("{data}");
            } else {
                println!("{}m -> {}", mins_till(nxt.kezdet_idopont), nxt.nev);
            }
        }
        for cnt_lsn in current_lessons(&lessons) {
            if json {
                let data = serde_json::to_string(&(mins_till(cnt_lsn.veg_idopont), cnt_lsn))?;
                println!("{data}");
            } else {
                println!("{}, {}m", cnt_lsn.nev, mins_till(cnt_lsn.veg_idopont));
            }
        }
        return Ok(());
    }
    if json {
        let json = serde_json::to_string(&lessons)?;
        println!("{json}");
    } else {
        user.print_day(lessons, &lessons_of_week);
    }
    Ok(())
}

/// minutes `till` now
fn mins_till(till: LDateTime) -> i64 {
    (till - Local::now()).num_minutes()
}
/// Parse the day got as `argument`.
/// # errors
/// - day shifter contains invalid number.
/// - any datetime is invalid.
pub fn parse_day(day: &str) -> Result<NaiveDate, String> {
    let today = Local::now().date_naive();
    let date = day.replace(['/', '.'], "-");
    info!("parsing date: {date}");

    // Parse From String
    let pfs = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d");
    if let Ok(ymd) = pfs(&date) {
        Ok(ymd)
    } else if let Ok(md) = pfs(&format!("{}-{date}", today.year())) {
        Ok(md)
    } else if let Ok(d) = pfs(&format!("{}-{}-{date}", today.year(), today.month())) {
        Ok(d)
    } else {
        info!("day shifter");
        let day_shift = date
            .parse::<i16>()
            .map_err(|e| format!("invalid day shifter: {e:?}"))?;
        let day = today + TimeDelta::days(day_shift.into());
        Ok(day)
    }
}

/// Returns the current [`Lesson`]s of this [`User`] from `lessons` which shall include today's [`Lesson`]s.
/// # Warning
/// returns a `Vec<&Lesson>`, as a person might accidentally have more than one lessons at a time
pub fn current_lessons(lessons: &[Lesson]) -> Vec<&Lesson> {
    lessons.iter().filter(|lsn| lsn.happening()).collect()
}
/// Returns the next [`Lesson`] of this [`User`] from `lessons` which shall include today's [`Lesson`]s.
/// # Warning
/// There might accidentally be more next [`Lesson`]s. In this case only one of them is returned.
/// Also, if there is any `current_lesson`, None is returned
pub fn next_lesson(lessons: &[Lesson]) -> Option<&Lesson> {
    if !current_lessons(lessons).is_empty() {
        return None;
    }
    lessons
        .iter()
        .find(|lsn| lsn.forecoming() && !ignore_lesson(lsn))
}
/// whether it's fake or cancelled
fn ignore_lesson(lsn: &Lesson) -> bool {
    lsn.kamu_smafu() || lsn.cancelled() || lsn.nev == EMPTY_NAME
}

/// you may want to check `lsn` validity: `lsn.kamu_smafu()`
pub fn disp(lsn: &Lesson, past_lessons: &[Lesson], test: Option<&AnnouncedTest>) -> Vec<String> {
    let topic = lsn
        .tema
        .as_ref()
        .map(|t| [": ", &t.italic().to_string()].concat())
        .unwrap_or_default();
    let name = format!("{}{topic}", lsn.nev);
    let name = if lsn.cancelled() {
        let past_morpheme = if lsn.forecoming() { "" } else { "t" };
        format!("elmarad{past_morpheme}: {name}").red().to_string()
    } else {
        name
    };
    let room = lsn
        .clone()
        .terem_neve
        .unwrap_or_default()
        .replace("terem", "")
        .trim()
        .to_string();
    let teacher = if let Some(sub_teacher) = &lsn.helyettes_tanar_neve {
        format!("helyettes: {}", sub_teacher.underline())
    } else {
        lsn.tanar_neve.clone().unwrap_or_default()
    };
    let mins_to_start = mins_till(lsn.kezdet_idopont);
    let from = if next_lesson(past_lessons).is_some_and(|nxt| nxt == lsn) && mins_to_start < 120 {
        format!("{mins_to_start} perc").yellow().to_string()
    } else {
        lsn.kezdet_idopont.format("%H:%M").to_string()
    };
    let to = if lsn.happening() {
        let till_end = mins_till(lsn.veg_idopont);
        format!("{till_end} perc").cyan().to_string()
    } else {
        lsn.veg_idopont.format("%H:%M").to_string()
    };
    let date_time = [from, to].join(" - ");
    let num = lsn.idx().to_string();

    let mut row = vec![num, date_time, name, room, teacher];
    if lsn.absent() {
        row.push("hiányoztál".to_string());
    }
    if let Some(existing_test) = test {
        let topic = if let Some(topic) = existing_test.temaja.as_ref() {
            format!(": {}", topic.italic())
        } else {
            String::new()
        };
        let test = format!("{}{}", existing_test.modja.leiras.bold(), topic);
        row.push(test);
    }

    row
}

impl User {
    /// print all lessons of a day
    pub fn print_day(&self, mut lessons: Vec<Lesson>, lessons_of_week: &[Lesson]) {
        let Some(first_lesson) = lessons.first() else {
            warn!("empty lesson-list got, won't print");
            return;
        };
        let day_start = first_lesson.kezdet_idopont;
        let day = first_lesson.datum.date_naive();
        let header = if first_lesson.kamu_smafu() {
            lessons.remove(0).nev.clone()
        } else {
            format!("{}, {}", day_start.hun_day_of_week(), day_start.pretty())
        };
        println!("{header}");
        if lessons.is_empty() {
            return;
        } // in the unfortunate case of stupidity

        let tests = self.get_tests((Some(day), Some(day))).unwrap_or_default();

        let mut table = ascii_table::AsciiTable::default();
        #[rustfmt::skip]
        let headers = [".", "ekkor", "tantárgy", "terem", "tanár", "extra", "extra-extra"];
        for (i, head) in headers.into_iter().enumerate() {
            table.column(i).set_header(head);
        }

        let mut data = vec![];
        let first_n = u8::from(lessons[0].idx() == 1); // school starts with 1 or 0
        for (ix, lsn) in lessons.iter().enumerate() {
            let cnt_n = lsn.idx(); // this is the `n`. lesson of the day
            let prev_ix = ix.wrapping_sub(1); // index of the previous lesson in the vector

            let wrong_n = |prev: &Lesson| prev.idx() != cnt_n - 1;
            if (ix == 0 && cnt_n != first_n) || lessons.get(prev_ix).is_some_and(wrong_n) {
                let prev_n = cnt_n.wrapping_sub(1);
                let empty = get_empty(prev_n, lessons_of_week);
                let mut empty_disp = disp(&empty, lessons_of_week, None);
                for item in &mut empty_disp {
                    *item = item.dim().to_string();
                }
                data.push(empty_disp);
            }
            let same_n = |t: &&AnnouncedTest| t.orarendi_ora_oraszama == lsn.oraszam;
            let ancd_test = tests.iter().find(same_n);
            let row = disp(lsn, lessons_of_week, ancd_test);
            data.push(row);
        }
        table.print(data);
    }
}

/// name given to an empty lesson
const EMPTY_NAME: &str = "lukas";

/// create a good-looking empty lesson, using the given properties
fn get_empty(n: u8, ref_lessons: &[Lesson]) -> Lesson {
    let irval = nth_lesson_when(n, ref_lessons);
    Lesson {
        nev: EMPTY_NAME.to_string(),
        tema: Some(String::from("lazíts!")),
        oraszam: Some(n),
        kezdet_idopont: irval.0.unwrap_or_default(),
        veg_idopont: irval.1.unwrap_or_default(),
        ..Default::default()
    }
}

/// When could this (empty) lesson take place?
fn nth_lesson_when(n: u8, ref_lessons: &[Lesson]) -> (Option<LDateTime>, Option<LDateTime>) {
    let same_n = |l: &&Lesson| l.oraszam.is_some_and(|ln| ln == n);
    let extract_irval = |j: &Lesson| (j.kezdet_idopont, j.veg_idopont);
    ref_lessons.iter().find(same_n).map(extract_irval).unzip()
}

pub fn default_day(user: &User) -> NaiveDate {
    warn!("searching for a suitable day to show the timetable for");
    let now = Local::now();
    let today = now.date_naive();

    let mut skip_days = TimeDelta::days(0); // starting with today
    while let Ok(lsns) = user.get_timetable(today + skip_days, true)
    // summertime sadness, stop
        && !lsns.is_empty()
    {
        if let Some(nxt_lsn) = lsns
            .iter() // modified version of `next_lesson`, allowing `current_lessons`
            .find(|lsn| !ignore_lesson(lsn) && (lsn.happening() || lsn.forecoming()))
        {
            return nxt_lsn.kezdet_idopont.date_naive(); // day of next lesson
        }
        skip_days += TimeDelta::days(7); // check out next week
    }
    today // fallback
}
