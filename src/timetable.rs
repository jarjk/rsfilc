//! lessons the student has

use crate::{time::MyDate, user::User, utils};
use chrono::{Datelike, Local, NaiveDate, TimeDelta};
use ekreta::{AnnouncedTest, Lesson, Res};
use log::*;
use yansi::Paint;

pub fn handle(day: NaiveDate, user: &User, current: bool, week: bool, json: bool) -> Res<()> {
    let lessons_of_week = user.get_timetable(day, true)?;
    let lessons = user.get_timetable(day, false)?; // PERF: reuses cache from previous line's network fetch
    if week && lessons_of_week.is_empty() && !json {
        println!("ezen a héten nincs rögzített órád, juhé!");
    } else if !week && lessons.is_empty() && !json {
        println!("{day} ({}) nincs rögzített órád, juhé!", day.weekday());
    }
    if current {
        if let Some(nxt) = next_lesson(&lessons_of_week) {
            if json {
                let data = serde_json::to_string(&(nxt.mins_till_start(), nxt))?;
                println!("{data}");
            } else {
                println!("{}m -> {}", nxt.mins_till_start(), nxt.nev);
            }
        }
        for cnt_lsn in current_lessons(&lessons) {
            if json {
                let data = serde_json::to_string(&(cnt_lsn.mins_till_end(), cnt_lsn))?;
                println!("{data}");
            } else {
                println!("{}, {}m", cnt_lsn.nev, cnt_lsn.mins_till_end());
            }
        }
        return Ok(());
    }
    if json {
        let print_lsns = if week { lessons_of_week } else { lessons };
        let json = serde_json::to_string(&print_lsns)?;
        println!("{json}");
    } else if week {
        user.print_week(lessons_of_week);
    } else {
        user.print_day(lessons);
    }

    Ok(())
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
/// Also, if there is any `current_lesson`, [`None`] is returned
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
    lsn.kamu_smafu() || lsn.cancelled()
}

/// you may want to check `lsn` validity: `lsn.kamu_smafu()`
pub fn disp(lsn: &Lesson, nxt_lsn: &Option<Lesson>, test: Option<&AnnouncedTest>) -> Vec<String> {
    let topic = lsn
        .tema
        .as_ref()
        .map(|t| [": ", &t.italic().to_string()].concat())
        .unwrap_or_default();
    let name = format!("{}{topic}", lsn.nev.bold());
    let name = if lsn.cancelled() {
        let past_morpheme = if lsn.forecoming() { "" } else { "t" };
        format!("elmarad{past_morpheme}: {name}").red().to_string()
    } else {
        name
    };
    let room = lsn.normalised_room().italic().to_string();
    let teacher = if let Some(sub_teacher) = &lsn.helyettes_tanar_neve {
        format!("helyettes: {}", sub_teacher.underline())
    } else {
        lsn.tanar_neve.clone().unwrap_or_default()
    };
    let mins_to_start = lsn.mins_till_start();
    let from = if nxt_lsn.as_ref().is_some_and(|nxt| nxt == lsn) && mins_to_start < 120 {
        format!("{mins_to_start} perc").yellow().to_string()
    } else {
        lsn.kezdet_idopont.format("%H:%M").to_string()
    };
    let to = if lsn.happening() {
        let till_end = lsn.mins_till_end();
        format!("{till_end} perc").cyan().to_string()
    } else {
        lsn.veg_idopont.format("%H:%M").to_string()
    };
    let date_time = [from, to].join(" - ");
    let num = lsn.d_num().to_string();

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
    pub fn print_day(&self, mut lessons: Vec<Lesson>) {
        let Some(first_lesson) = lessons.first() else {
            warn!("empty lesson-list got, won't print");
            return;
        };
        let day = first_lesson.date_naive();
        let header = if first_lesson.kamu_smafu() {
            lessons.remove(0).nev.clone()
        } else {
            let day_start = first_lesson.kezdet_idopont;
            format!("{}, {}", day_start.hun_day_of_week(), day_start.pretty())
        };
        println!("{header}");
        if lessons.is_empty() {
            return;
        } // in the unfortunate case of stupidity

        let tests = self.get_tests((Some(day), Some(day))).unwrap_or_default();
        let (day_start, mut data) = index_tt(&lessons);
        let nxt_lsn = next_lesson(&lessons).cloned();

        for lsn in lessons {
            let h_ix = usize::from(lsn.d_num() - day_start); // hour index

            let same_n = |t: &&AnnouncedTest| t.orarendi_ora_oraszama == lsn.oraszam;
            let ancd_test = tests.iter().find(same_n);
            data[h_ix] = disp(&lsn, &nxt_lsn, ancd_test);
        }

        #[rustfmt::skip]
        utils::print_table_wh([".", "EKKOR", "ÓRA", "TEREM", "TANÁR", "EXTRA", "EXTRA-EXTRA"], data);
    }

    /// print week timetable
    fn print_week(&self, mut lsns_week: Vec<Lesson>) {
        lsns_week.retain(|l| !l.kamu_smafu()); // delete fake lessons
        if lsns_week.is_empty() {
            return;
        }

        let (day_start, mut data) = index_tt(&lsns_week);
        let mut prev_d = lsns_week[0].date_naive(); // previous day
        let mut d_ix = 1; // day index
        let nxt_lsn = next_lesson(&lsns_week).cloned();

        for lsn in lsns_week {
            if lsn.date_naive() != prev_d {
                prev_d = lsn.date_naive();
                d_ix += 1; // next day
            }

            let h_ix = usize::from(lsn.d_num() - day_start); // hour index
            while data[h_ix].get(d_ix).is_none() {
                data[h_ix].push(String::new()); // new column for this day
            }
            let subj = if lsn.happening() {
                lsn.nev.cyan()
            } else if nxt_lsn
                .as_ref()
                .is_some_and(|nl| nl == &lsn && lsn.mins_till_start() < 24 * 60)
            {
                lsn.nev.yellow()
            } else if lsn.cancelled() {
                lsn.nev.red()
            } else if lsn.absent() {
                lsn.nev.on_red()
            } else if lsn.helyettes_tanar_neve.is_some() {
                lsn.nev.on_yellow()
            } else if lsn.bejelentett_szamonkeres_uid.is_some() {
                lsn.nev.on_blue()
            } else {
                lsn.nev.resetting()
            };
            data[h_ix][d_ix] = format!("{} {}", subj.bold(), lsn.normalised_room().italic().dim());
        }
        #[rustfmt::skip]
        utils::print_table_wh([".", "HÉTFŐ", "KEDD", "SZERDA", "CSÜTÖRTÖK", "PÉNTEK", "SZOMBAT"], data);
    }
}

/// # SAFETY
/// make sure `lessons` is not empty
fn index_tt(lessons: &[Lesson]) -> (u8, Vec<Vec<String>>) {
    let first_h_ix = lessons.first().map(Lesson::d_num).unwrap();
    let max_h_ix = lessons.iter().map(Lesson::d_num).max().unwrap();
    let day_start = u8::from(first_h_ix != 0); // day shall start on `day_start`th lesson

    let h_max = usize::from(max_h_ix - day_start + 1); // hour max: last `h_ix` of the day
    let mut data = vec![vec![String::new(); 1]; h_max]; // (index-column + one for sure =) 2 column * `h_max` rows => timetable
    for ix in day_start..=max_h_ix {
        data[usize::from(ix - day_start)][0] = ix.to_string(); // index-column
    }
    (day_start, data)
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
            return nxt_lsn.date_naive(); // day of next lesson
        }
        skip_days += TimeDelta::days(7); // check out next week
    }
    today // fallback
}
