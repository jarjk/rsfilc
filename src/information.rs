//! basic User info, `Kréta` stores

use crate::utils;
use ekreta::{Res, UserInfo};

/// `user_data`: (`user_info`, `id`, `default_id`)
fn disp(user_data: &(UserInfo, &String, &str)) -> Vec<String> {
    let id = user_data.1.to_string();
    let def_usr = if id == user_data.2 { "Alap: " } else { "" };
    let info = &user_data.0;
    let name = format!("{def_usr}{}", info.nev);
    let school = info.intezmeny_nev.clone();
    let birth = info.szuletesi_datum.format("%Y.%m.%d").to_string();
    vec![name, id, school, birth]
}

pub fn handle<'a, I>(def_userid: &str, users: I, args: &crate::Args) -> Res<()>
where
    I: Iterator<Item = &'a crate::User>,
{
    let data = users
        .map(|u| Ok((u.get_userinfo()?, &u.userid, def_userid)))
        .collect::<Res<Vec<_>>>()?;

    let headers = ["NÉV", "OM AZONOSÍTÓ", "OSKOLA", "SZÜLETETT"].into_iter();
    let to_str = if args.machine { None } else { Some(disp) };
    utils::print_table(&data, headers, args.reverse, args.number, to_str)
}
