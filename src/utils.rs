use ekreta::{OptIrval, Res};
use log::{debug, info};
use yansi::Paint;

/// use `cache_t` as `interval.0` (from) if some
pub fn fix_from(cache_t: Option<ekreta::LDateTime>, mut irval: OptIrval) -> OptIrval {
    debug!("got interval: {irval:?}");
    if let Some(ct) = cache_t.map(|ct| ct.date_naive())
        && irval
            .0
            .is_none_or(|from| from < ct && irval.1.is_none_or(|to| to > ct))
    {
        info!("from cached, replacing {:?} to {ct:?}", irval.0);
        irval.0 = Some(ct);
    }
    irval
}
/// convert type name of `T` to a kind name, used for cache
pub fn type_to_kind_name<T>() -> Res<String> {
    let type_name = std::any::type_name::<T>();
    let kind = type_name.split("::").last().ok_or("invalid type_name")?;
    let kind = kind.trim_matches(['<', '>']).to_ascii_lowercase();
    Ok(kind)
}

#[macro_export]
/// generate get fn named `fn_name` for type `ep`, specify whether
/// content once `cached_can_change` or not then sort with `sorting`
macro_rules! gen_get_for {
    ($fn_name:ident, $ep:ty, $cached_can_change:expr, $sorting:expr) => {
        /// get all items between `from` and `to`
        /// # Errors
        /// net
        pub fn $fn_name(&self, interval: OptIrval) -> Res<Vec<$ep>> {
            self.load_n_fetch::<$ep>(interval.clone(), !$cached_can_change)
                .map(|mut items| {
                    $sorting(&mut items);
                    if interval.0.is_none() {
                        self.store_cache(&items)?;
                    }
                    Ok(items)
                })?
        }
    };
}

/// print table with headers
pub fn print_table_wh<I: IntoIterator<Item: ToString>>(headers: I, data: Vec<Vec<String>>) {
    let mut tbl = ascii_table::AsciiTable::default();
    for (i, h) in headers.into_iter().enumerate() {
        tbl.column(i).set_header(h.to_string().bold().to_string());
    }
    tbl.print(data);
}

/// print `num` `items` using `to_str`, reversed if `rev` otherwise not
pub fn print_table<T, I, F>(
    items: &[T],
    headers: I,
    rev: bool,
    num: usize,
    to_str: Option<F>,
) -> Res<()>
where
    T: serde::Serialize,
    I: Iterator<Item: ToString>,
    F: Fn(&T) -> Vec<String>,
{
    if items.is_empty() {
        return Ok(());
    }
    let iter: Box<dyn Iterator<Item = &T>> = if rev {
        Box::new(items.iter().rev())
    } else {
        Box::new(items.iter())
    };
    if let Some(to_str) = to_str {
        let data = iter.take(num).map(to_str).collect::<Vec<_>>();
        print_table_wh(headers, data);
    } else {
        let data = serde_json::to_string(&iter.take(num).collect::<Vec<_>>())?;
        println!("{data}");
    }
    Ok(())
}

/// set `field` to true if `name` env var exists, and is not set to "0"
pub fn bool_from_env(name: &str, field: &mut bool) {
    if let Ok(value) = std::env::var(name) {
        if value == "0" {
            *field = false;
        } else {
            *field = true;
        }
    }
    // TODO less comments?
    // else don't touch field
    // if we get here, its likely a `VarError`, the env var wasn't set
    // in that case, we ignore it, it will be set based on its value in the config file
}
