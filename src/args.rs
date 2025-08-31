//! CLI arguments

use clap::{Parser, Subcommand};
use log::info;
use std::io::{self, IsTerminal};

/// default number of entries to show
const NUM: usize = usize::MAX;

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,
    /// produce machine-readable output, mostly json
    #[arg(short, long, num_args(1), visible_alias = "json", default_value_t = !io::stdout().is_terminal())]
    pub machine: bool,
    /// reverse the order of entries, WARN: will be ignored by some commands
    #[arg(short, long, default_value_t = false)]
    pub reverse: bool,
    /// maximum number of entries to show, WARN: will be ignored by some commands
    #[arg(short, long, default_value_t = NUM)]
    pub number: usize,
    /// enable verbose logging into the log file
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
    /// show cache dir
    #[arg(long, default_value_t = false)]
    pub cache_dir: bool,
    /// show config path
    #[arg(long, default_value_t = false)]
    pub config_path: bool,
    /// Manually set a user (by name or ID) for a command
    #[arg(long, env = "RSFILC_USER")]
    pub user: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// starts the Text User Interface
    Tui {},
    /// generate completions for <SHELL>
    Completions { shell: clap_complete::Shell },

    /// information about lessons, today by default
    #[clap(visible_alias = "tt")]
    Timetable {
        /// which day to show: `(+|-)n` (`n` is the number of days added to today) or [YYYY-][MM-][DD]
        #[arg(value_parser = crate::timetable::parse_day)]
        day: Option<chrono::NaiveDate>,

        /// show current lesson if any
        #[arg(short, long, default_value_t = false)]
        current: bool,

        /// week overview
        #[arg(short, long, default_value_t = false, conflicts_with = "current")]
        week: bool,
    },

    /// evaluations/grades the user received
    #[clap(visible_alias = "e")]
    Evals {
        /// filter by `subject`
        #[arg(short, long)]
        subject: Option<String>,
        /// filter by `kind` eg. témazáró, or `title`
        #[arg(short, long)]
        filter: Option<String>,
        /// calculate average
        #[arg(short, long, default_value_t = false)]
        average: bool,
        /// ghost evals
        #[arg(requires = "average")]
        ghost: Vec<u8>,
    },

    /// messages the user either received or sent
    #[clap(visible_alias = "msg")]
    Messages {
        /// show additional notes/system messages
        #[arg(long, default_value_t = false)]
        notes: bool,
        /// id of the message to render
        id: Option<isize>,
    },

    /// information about lessons the user missed
    #[clap(visible_alias = "a")]
    Absences {
        /// filter the subject to show
        #[arg(short, long)]
        subject: Option<String>,
        /// count the number of absences
        #[arg(short, long, default_value_t = false)]
        count: bool,
    },

    /// information about forecoming exams/tests
    #[clap(visible_alias = "t")]
    Tests {
        /// filter the subject to show
        #[arg(short, long)]
        subject: Option<String>,
        /// show tests from the past as well
        #[arg(short, long, default_value_t = false)]
        past: bool,
    },

    /// managing users of this program, listing if nothing specified
    #[clap(visible_alias = "u")]
    User {
        /// the id or name of the user used for args
        userid: Option<String>,
        /// log out of an account
        #[arg(long, default_value_t = false, conflicts_with_all = ["login", "switch"], requires = "userid")]
        logout: bool,
        /// log in to an existing E-Kréta account
        #[arg(long, default_value_t = false, requires = "userid")]
        login: bool,
        /// switch between existing accounts
        #[rustfmt::skip]
        #[arg(short, long, default_value_t = false, conflicts_with = "login", requires = "userid")]
        switch: bool,
        /// print the cache directory for a user
        #[arg(long, default_value_t = false, conflicts_with_all = ["logout", "login", "switch"])]
        cache_dir: bool,
    },

    /// information about all schools in the `Kréta` database
    #[clap(visible_alias = "s")]
    Schools {
        /// search for school
        #[arg(short, long, name = "SCHOOL_PROPERTY")]
        search: Option<String>,
    },
    /// show the time of next server downtime
    NextDowntime,
    /// guided renaming
    Rename,
}
impl Command {
    pub fn user_needed(&self) -> bool {
        info!("checking whether user is needed for task");
        if let Command::User {
            logout,
            login,
            switch,
            cache_dir,
            userid: _,
        } = &self
        {
            // we do need one on: nothing, switching, listing
            let nothing_specified = !logout && !login && !switch && !cache_dir;
            return nothing_specified || *switch;
        }
        !matches!(
            self,
            Command::Schools { search: _ } | Command::Completions { shell: _ }
        )
    }
}
