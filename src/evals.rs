//! evaluations/grades the user received

use crate::{time::MyDate, user::User, utils};
use ekreta::{Evaluation, Res};
use log::info;

pub fn handle(
    user: &User,
    filter: Option<String>,
    subj: Option<String>,
    ghost: &[u8],
    avg: bool,
    args: &crate::Args,
) -> Res<()> {
    let mut evals = user.get_evals((None, None))?;
    info!("got evals");
    if let Some(kind) = filter {
        filter_by_kind_or_title(&mut evals, &kind);
    }
    if let Some(subject) = subj {
        filter_by_subject(&mut evals, &subject);
    }
    if avg {
        let avg = calc_average(&evals, ghost);
        println!("Average: {avg:.2}");

        return Ok(());
    }
    #[rustfmt::skip]
    let headers = ["TÉMA", "JEGY", "TANTÁRGY", "TÍPUS", "TANÁR", "IDŐPONT"];
    let disp = if args.machine { None } else { Some(display) };
    utils::print_table(&evals, headers.into_iter(), args.reverse, args.number, disp)
}

/// Filter `evals` by `kind`
pub fn filter_by_kind_or_title(evals: &mut Vec<Evaluation>, filter: &str) {
    let filter = filter.to_lowercase();
    log::info!("filtering evals by kind: {filter}");
    evals.retain(|eval| {
        eval.r#mod
            .as_ref()
            .is_some_and(|m| m.leiras.to_lowercase().contains(&filter))
            || eval
                .tema
                .as_ref()
                .is_some_and(|t| t.to_lowercase().contains(&filter))
            || eval.tipus.leiras.to_lowercase().contains(&filter)
    });
}

/// Filter `evals` by `subject`
pub fn filter_by_subject(evals: &mut Vec<Evaluation>, filter: &str) {
    log::info!("filtering evals by subject: {filter}");
    let subjs: Vec<&str> = filter.split(|c| c == '|').collect();

    evals.retain(|eval| {
        let nev = eval.tantargy.nev.to_lowercase();
        subjs.iter().any(|subj| nev.contains(subj))
    });

    if subjs.len() > 1 {
        evals.sort_by(|a, b| a.tantargy.nev.cmp(&b.tantargy.nev));
    }
}

/// Calculate average of `evals` and `ghosts` evals
pub fn calc_average(evals: &[Evaluation], ghosts: &[u8]) -> f32 {
    log::info!("calculating average for evals");
    let evals = evals
        .iter()
        .filter(|eval| !eval.evvegi() && !eval.felevi() && eval.szam_ertek.is_some());

    // filter it, so only valid grades retain
    let ghosts = ghosts.iter().filter(|g| *g > &0 && *g <= &5);

    let sum = evals.clone().fold(0., |sum, cur| {
        sum + f32::from(cur.szam_ertek.unwrap_or(0)) * cur.szorzo()
    }) + ghosts.clone().fold(0., |sum, num| sum + f32::from(*num));

    let count = evals.clone().fold(0., |sum, cur| sum + cur.szorzo()) + ghosts.count() as f32;

    sum / count
}

fn display(eval: &Evaluation) -> Vec<String> {
    let desc = eval.tema.clone().unwrap_or_default();
    let grade = if let Some(num) = eval.szam_ertek {
        num.to_string()
    } else {
        eval.szoveges_ertek.clone()
    };
    let subj_name = eval.tantargy.nev.clone();
    let how_desc = eval.r#mod.clone().map(|l| l.leiras).unwrap_or_default();
    // let kind_name = eval.tipus.leiras.replace(" jegy/értékelés", "");
    let teacher = eval.ertekelo_tanar_neve.clone().unwrap_or_default();
    let when = eval.keszites_datuma.pretty();

    vec![
        desc, grade, subj_name, how_desc, /* kind_name, */ teacher, when,
    ]
}
