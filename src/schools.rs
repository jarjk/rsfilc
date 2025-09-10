//! every school that uses the `Kréta` system

use crate::{Res, cache, utils};
use log::info;

pub fn handle(search: Option<String>, args: &crate::Args) -> Res<()> {
    let mut schools = get()?;
    if let Some(school_name) = search {
        filter(&mut schools, &school_name);
    }
    info!("listing schools");
    // utils::print_them_basic(schools.iter(), disp);
    let headers = ["NÉV", "AZONOSÍTÓ", "TELEPÜLÉS"].into_iter();
    let disp = if args.machine { None } else { Some(display) };
    utils::print_table(&schools, headers, args.reverse, args.number, disp)
}

pub fn get() -> Res<Vec<ekreta::School>> {
    let cached = cache::load("", "schools");
    if let Some((_t, content)) = cached {
        log::info!("loading schools from cache");
        let cached_schools = serde_json::from_str(&content)?;
        return Ok(cached_schools);
    }
    let resp = ekreta::School::fetch_schools_resp()?;

    log::info!("received schools from refilc api");
    let json = &resp.into_body().read_to_string()?;
    let vec = serde_json::from_str(json)?;
    cache::store("", "schools", json)?;
    Ok(vec)
}

pub fn filter(schools: &mut Vec<ekreta::School>, search_for: &str) {
    log::info!("searching for {search_for} in schools");
    schools.retain(|school| {
        [
            school.nev.clone(),
            school.telepules.clone(),
            school.azonosito.clone(),
        ]
        .iter()
        .any(|j| j.to_lowercase().contains(&search_for.to_lowercase()))
    });
}

fn display(school: &ekreta::School) -> Vec<String> {
    vec![
        school.nev.clone(),
        school.azonosito.clone(),
        school.telepules.clone(),
    ]
}
