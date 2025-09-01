//! Absences

use crate::{time::MyDate, user::User, utils};
use ekreta::{Absence, Res};

pub fn handle(user: &User, subj: Option<String>, count: bool, args: &crate::Args) -> Res<()> {
    let mut absences = user.get_absences((None, None))?;
    if let Some(subject) = subj {
        filter_by_subject(&mut absences, &subject);
    }
    if count {
        let unver = absences.iter().filter(|item| !item.igazolt()).count();
        if args.machine {
            println!("{{\"verified\":{},\"unverified\":{unver}}}", absences.len());
        } else {
            println!("Összes hiányzásod száma: {}", absences.len());
            println!("Ebből még igazolatlan: {unver}");
        }
        return Ok(());
    }
    #[rustfmt::skip]
    let headers = ["TANTÁRGY", "TANÁR", "ETTŐL", "EDDIG", "ENNYIT", "IGAZOLÁS TIPUS"].into_iter();
    let disp = if args.machine { None } else { Some(display) };
    utils::print_table(&absences, headers, args.reverse, args.number, disp)
}

/// filter [`Abs`]ences by `subj`ect
pub fn filter_by_subject(abss: &mut Vec<Absence>, subj: &str) {
    log::info!("filtering absences by subject: {subj}");
    abss.retain(|abs| {
        abs.tantargy
            .nev
            .to_lowercase()
            .contains(&subj.to_lowercase())
    });
}

fn display(abs: &Absence) -> Vec<String> {
    let from = abs.ora.kezdo_datum.pretty();
    let to = abs.ora.veg_datum.pretty();
    let subj = abs.tantargy.nev.clone();
    let teacher = abs.rogzito_tanar_neve.clone();
    let lateness = if let Some(min) = &abs.keses_percben {
        format!("késtél {min} percet")
    } else {
        abs.igazolas_allapota.to_lowercase().replace("do", "dó")
    };
    let kind = abs.igazolas_tipusa.clone().unwrap_or_default().leiras;

    vec![subj, teacher, from, to, lateness, kind]
}
