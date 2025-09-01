//! Announced tests

use crate::{time::MyDate, user::User, utils};
use ekreta::{AnnouncedTest, Res};

pub fn handle(past: bool, user: &User, subj: Option<String>, args: &crate::Args) -> Res<()> {
    #[rustfmt::skip]
    let from = if past { None } else { Some(chrono::Local::now().date_naive()) };
    let mut all_announced = user.get_tests((from, None))?;
    if let Some(subject) = subj {
        filter_by_subject(&mut all_announced, &subject);
    }
    let headers = ["TÉMA", "TANTÁRGY", "DÁTUM", "MÓD", "TANÁR"].into_iter();
    let dix = if args.machine { None } else { Some(display) };
    utils::print_table(&all_announced, headers, args.reverse, args.number, dix)
}

/// filter [`Ancd`] tests by `subj`ect
pub fn filter_by_subject(ancds: &mut Vec<AnnouncedTest>, subj: &str) {
    log::info!("filtering announced tests by subject: {subj}");
    ancds.retain(|ancd| {
        ancd.tantargy_neve
            .to_lowercase()
            .contains(&subj.to_lowercase())
    });
}

fn display(ancd: &AnnouncedTest) -> Vec<String> {
    let about = ancd.temaja.clone().unwrap_or_default();
    let subj = ancd.tantargy_neve.clone();
    let date = ancd.datum.pretty();
    let kind = ancd.modja.leiras.clone();
    let teacher = ancd.rogzito_tanar_neve.clone();

    vec![about, subj, date, kind, teacher]
}
