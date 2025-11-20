use args::{Args, Command};
use clap::{CommandFactory, Parser};
use config::{CONFIG, Config};
use ekreta::Res;
use inquire::{Confirm, MultiSelect, Text};
use log::*;
use std::{collections::BTreeSet, fs::OpenOptions, mem};
use time::MyDate;
use user::User;

mod absences;
mod announced;
mod args;
mod cache;
mod config;
mod evals;
mod information;
mod messages;
mod paths;
mod schools;
mod time;
mod timetable;
mod user;
mod utils;

fn main() -> Res<()> {
    // parse args
    let cli_args = Args::parse();
    // set up fern
    set_up_logger(cli_args.verbosity)?;

    // handle cli args and execute program
    run(cli_args)?;

    Ok(())
}

fn run(args: Args) -> Res<()> {
    if args.command.is_none() {
        if args.cache_dir {
            let cache_dir = paths::cache_dir("").ok_or("no cache dir found")?;
            println!("{}", cache_dir.display());
            return Ok(());
        }
        if args.config_path {
            println!("{}", Config::path()?.display());
            return Ok(());
        }
    }
    let command = args
        .command
        .as_ref()
        .unwrap_or(&Command::Timetable {
            day: None,
            current: false,
            week: false,
        })
        .clone();
    // have a valid user
    let user = if command.user_needed() {
        if let Some(who) = args.user.as_ref() {
            User::load(&CONFIG, who).ok_or(format!("invalid user ({who}) specified"))?
        } else {
            User::load(&CONFIG, &CONFIG.default_userid)
                .ok_or("no user found, please log in to an account with `rsfilc user --login`")?
        }
    } else {
        User::default()
    };

    match command {
        Command::Completions { shell: sh } => {
            info!("creating shell completions for {sh}");
            clap_complete::generate(sh, &mut Args::command(), "rsfilc", &mut std::io::stdout());
            Ok(())
        }
        Command::Timetable { day, current, week } => {
            info!("requested {}: {day:?}", if week { "week" } else { "day" });
            let day = day.unwrap_or_else(|| timetable::default_day(&user));
            info!("showing {}: {day}", if week { "week" } else { "day" });
            timetable::handle(day, &user, current, week, args.machine)
        }

        Command::Evals {
            subject: subj,
            filter,
            average,
            ghost,
        } => evals::handle(&user, filter, subj, &ghost, average, &args),

        Command::Messages { notes, id } => {
            if notes {
                messages::handle_note_msgs(&user, id, &args)
            } else {
                messages::handle(&user, id, &args)
            }
        }

        Command::Absences { count, subject } => absences::handle(&user, subject, count, &args),

        Command::Tests { subject, past } => announced::handle(past, &user, subject, &args),

        Command::User {
            logout,
            login,
            switch,
            cache_dir,
            userid,
        } => user::handle(userid, login, logout, switch, cache_dir, &args),

        Command::Schools { search } => schools::handle(search, &args),

        Command::NextDowntime => {
            let next_downt = user.get_userinfo()?.next_downtime();
            let probably_now = next_downt < chrono::Local::now();
            if args.machine {
                println!("{{\"next_downtime\":\"{next_downt}\"}}");
            } else {
                let now = if probably_now { ", probably ATM" } else { "" };
                println!("time of next server downtime: {}{now}", next_downt.pretty());
            }
            Ok(())
        }
        Command::Rename => guided_renames(&user),
    }
}

fn set_up_logger(verbosity: LevelFilter) -> Res<()> {
    let path = paths::cache_dir("")
        .ok_or("no cache dir")?
        .join(config::APP_NAME)
        .with_extension("log");
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {} {message}",
                chrono::Local::now(),
                record.level(),
                record.target(),
            ));
        })
        .level(verbosity)
        .chain(OpenOptions::new().create(true).append(true).open(path)?)
        .apply()?;
    Ok(())
}

fn guided_renames(user: &User) -> Res<()> {
    let mut conf = CONFIG.clone(); // don't use plain CONFIG afterwards
    let mut renames_already = mem::take(&mut conf.rename); // taken, newly fetched data won't get renames

    let today = chrono::Local::now().date_naive();
    let prev_w = today - chrono::TimeDelta::weeks(1);
    let next_w = today + chrono::TimeDelta::weeks(1);
    let mut tt = user.get_timetable(today, true).unwrap_or_default();
    tt.append(&mut user.get_timetable(prev_w, true).unwrap_or_default());
    tt.append(&mut user.get_timetable(next_w, true).unwrap_or_default());

    let mut to_rename = BTreeSet::new();
    let mut insert_if_some = |opt_item: Option<String>| {
        if let Some(item) = opt_item {
            to_rename.insert(item);
        }
    };
    for lsn in tt {
        insert_if_some(lsn.tantargy.map(|s| s.nev));
        insert_if_some(lsn.tanar_neve);
        insert_if_some(lsn.helyettes_tanar_neve);
        insert_if_some(lsn.terem_neve);
    }
    let to_rename = to_rename.into_iter().collect::<Vec<_>>();
    const PROMPT_MESSAGE: &str = "choose the ones you'd like to rename (Esc to skip)";
    let to_rename = MultiSelect::new(PROMPT_MESSAGE, to_rename).prompt()?;
    for mut rename in to_rename {
        let confirm = |message: String| {
            Confirm::new(&message)
                .with_default(false)
                .prompt_skippable()
                .map(|j| j.is_some_and(|c| c))
        };
        if let Some(already_to) = renames_already.get(&rename) {
            let message = format!("sure? '{rename}' is already replaced with '{already_to}'");
            if !confirm(message)? {
                continue;
            }
        } else if let Some((already_from, _already_to)) =
            renames_already.iter().find(|(_from, to)| **to == rename)
        {
            let message = format!("sure? '{rename}' is already replaced from '{already_from}'");
            if !confirm(message)? {
                continue;
            } else {
                rename = already_from.clone(); // don't rename `to` further, rename it's source
            }
        }

        let message = format!("replace '{rename}' to:");
        if let Ok(Some(to)) = Text::new(&message).prompt_skippable() {
            renames_already.insert(rename, to); // update
        }
    }
    conf.rename = renames_already;
    conf.save()?;
    Ok(())
}
