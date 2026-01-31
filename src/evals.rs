//! evaluations/grades the user received

use crate::{plotting::ChartBuilder, time::MyDate, user::User, utils};
use ekreta::{Evaluation, Res};
use log::info;

pub fn handle(
    user: &User,
    filter: Option<String>,
    subj: Option<String>,
    ghost: &[u8],
    avg: bool,
    plot: bool,
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
        if plot {
            let mut sum = 0.0f32;
            let points: Vec<(f32, f32)> = evals
                .iter()
                .filter(|eval| !eval.evvegi() && !eval.felevi())
                .filter_map(|eval| eval.szam_ertek)
                .chain(ghost.iter().filter(|g| *g > &0 && *g <= &5).copied())
                .enumerate()
                .map(|(i, eval)| {
                    sum += eval as f32;
                    let avg_i = sum / (i as f32 + 1.0);
                    (i as f32, avg_i)
                })
                .collect();

            let mut builder = ChartBuilder::new(points);
            builder.build_and_display(true, true, true, false);
        }
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
pub fn filter_by_subject(evals: &mut Vec<Evaluation>, subj: &str) {
    log::info!("filtering evals by subject: {subj}");
    evals.retain(|eval| {
        eval.tantargy
            .nev
            .to_lowercase()
            .contains(&subj.to_lowercase())
    });
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
