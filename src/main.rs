use args::{Args, Command};
use clap::{CommandFactory, Parser};
use config::Config;
use ekreta::Res;
use inquire::{Confirm, MultiSelect, Text};
use log::*;
use std::{collections::BTreeSet, env, fs::OpenOptions, mem};
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
    set_up_logger(cli_args.verbose)?;
    // load config from file, eg.: users
    let mut config = Config::load()?;

    // handle cli args and execute program
    run(cli_args, &mut config)?;

    Ok(())
}

fn run(args: Args, conf: &mut Config) -> Res<()> {
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
            User::load(conf, who).ok_or(format!("invalid user ({who}) specified"))?
        } else {
            User::load(conf, &conf.default_userid)
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
        Command::Tui {} => {
            warn!("TUI is not yet written");
            Err("TUI is to be written (soon)".into())
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
        } => user::handle(userid, login, conf, logout, switch, cache_dir, &args),

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
        Command::Rename => guided_renames(conf, &user),
    }
}

fn set_up_logger(verbose: bool) -> Res<()> {
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
        .level(if verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(OpenOptions::new().create(true).append(true).open(path)?)
        .apply()?;
    Ok(())
}

fn guided_renames(conf: &mut Config, user: &User) -> Res<()> {
    unsafe { env::set_var("NO_CACHE", "1") }; // SAFETY: this runs single-threaded
    unsafe { env::set_var("NO_RENAME", "1") };
    let tt = user.get_timetable(chrono::Local::now().date_naive(), true)?;
    let mut to_rename = BTreeSet::new();
    let mut renames_already = mem::take(&mut conf.rename);
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
    let to_rename = MultiSelect::new("choose the ones you'd like to rename", to_rename)
        .prompt_skippable()?
        .unwrap_or_default();
    for rename in to_rename {
        if let Some([_, already_to]) = renames_already.iter().find(|rn| rn[0] == rename) {
            let message = format!("sure? '{rename}' is already replaced with '{already_to}'");
            let should = Confirm::new(&message).with_default(false).prompt()?;
            if !should {
                continue;
            }
        }
        let message = format!("replace '{rename}' to:");
        if let Ok(Some(to)) = Text::new(&message).prompt_skippable() {
            renames_already.insert([rename, to]);
        }
    }
    conf.rename = renames_already;
    conf.save()?;
    Ok(())
}
