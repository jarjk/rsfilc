use crate::{Res, paths::cache_path};
use chrono::{DateTime, Local};
use std::fs::{self, File};
use std::io::Write;

/// save to disk
pub fn store(userid: &str, kind: &str, content: &str) -> Res<()> {
    let cp = cache_path(userid, kind).ok_or("couldn't get cache path")?;
    let mut f = File::create(&cp)?;
    log::info!("caching to {cp:?}");

    writeln!(f, "//{}", Local::now().to_rfc3339())?;
    writeln!(f, "{content}")?;

    Ok(())
}

/// load from disk
pub fn load(userid: &str, kind: &str) -> Option<(DateTime<Local>, String)> {
    let cp = cache_path(userid, kind)?;
    log::info!("loading cache from {cp:?}");
    if !cp.exists() {
        log::warn!("no saved cache exists");
        return None;
    }
    let content = fs::read_to_string(cp).ok()?;
    let mut cl = content.lines();
    let t = cl.next()?;
    // removing "//" (comment sequence)
    let t = DateTime::parse_from_rfc3339(&t[2..]).ok()?;

    let c = cl.next()?.to_string();

    Some((t.into(), c))
}
/// delete all cache and logs as well
pub fn delete_dir(userid: &str) -> Res<()> {
    if let Some(cd) = crate::paths::cache_dir(userid)
        && cd.exists()
    {
        log::warn!("deleting cache dir");
        fs::remove_dir_all(cd)?;
        log::info!("done");
    }
    Ok(())
}
