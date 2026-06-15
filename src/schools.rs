//! every school that uses the `Kréta` system

use crate::{Res, cache, utils};

pub fn handle(search: String, args: &crate::Args) -> Res<()> {
    let schools = get(&search)?;
    log::info!("listing schools");
    // utils::print_them_basic(schools.iter(), disp);
    let headers = ["NÉV", "AZONOSÍTÓ"].into_iter();
    let disp = if args.machine { None } else { Some(display) };
    utils::print_table(&schools, headers, args.reverse, args.number, disp)
}

pub fn get(q: &str) -> Res<Vec<ekreta::School>> {
    let cached = cache::load("", "schools");
    let mut cached_schools = Vec::new();
    if let Some((_t, content)) = cached {
        log::info!("loading schools from cache");
        cached_schools = serde_json::from_str(&content)?; // needs for recaching
        let filtered_cached_schools = filter(&cached_schools, q);
        if !filtered_cached_schools.is_empty() {
            return Ok(filtered_cached_schools);
        }
    }

    let schools = ekreta::School::fetch_schools(q)?;
    cached_schools.extend(schools.clone());
    cached_schools.dedup();
    let json = serde_json::to_string(&cached_schools)?;
    cache::store("", "schools", &json)?;
    Ok(schools)
}

pub fn filter(schools: &Vec<ekreta::School>, search_for: &str) -> Vec<ekreta::School> {
    log::info!("searching for {search_for} in schools");
    let mut filtered_schools = schools.clone();
    filtered_schools.retain(|school| {
        display(school)
            .concat()
            .to_lowercase()
            .contains(&search_for.to_lowercase())
    });
    return filtered_schools;
}

fn display(school: &ekreta::School) -> Vec<String> {
    vec![school.nev.clone(), school.azonosito.clone()]
}
