use crate::{config::Config, *};
use chrono::{Datelike, Local, NaiveDate, TimeDelta};
use ekreta::{
    Absence, Account, AnnouncedTest as Ancd, Evaluation as Eval, HeaderMap, LDateTime, Lesson,
    MsgItem, MsgOview, OptIrval, Token, consts, header,
};
use inquire::{Password, PasswordDisplayMode, Select};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

pub fn handle(
    userid: Option<String>,
    login: bool,
    logout: bool,
    switch: bool,
    cache_dir: bool,
    args: &crate::Args,
) -> Res<()> {
    let Some(name) = userid else {
        if cache_dir {
            let cache_dir =
                paths::cache_dir(&CONFIG.default_userid).ok_or("no cache dir found for user")?;
            println!("{}", cache_dir.display());
            return Ok(());
        }
        return information::handle(&CONFIG.default_userid, CONFIG.users.iter(), args);
    };
    let mut conf = CONFIG.clone(); // will mutate, don't use plain CONFIG afterwards
    if login {
        let res = User::login(name.clone(), &mut conf).inspect_err(|_| {
            eprintln!(
                "couldn't log in to user account, check your credentials and connection with Kréta"
            );
        });
        // delete cache dir if couldn't log in
        if res.is_err() {
            crate::cache::delete_dir(&name)?;
        }
        res?;
        println!("successful login");
        return conf.save();
    }
    let userid = conf
        .get_userid(name)
        .ok_or("the given userid/name isn't saved")?;
    if logout {
        conf.logout(userid);
        println!("logged out");
    } else if switch {
        conf.switch_user_to(&userid);
        println!("switched");
    } else if cache_dir {
        let cache_dir = paths::cache_dir(&userid).ok_or("no cache dir found for user")?;
        println!("{}", cache_dir.display());
    } else {
        let matching_users = conf.users.iter().filter(|u| u.userid == userid);
        information::handle(&conf.default_userid, matching_users, args)?;
    }
    conf.save()
}

/// Kréta, app user
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub userid: String,
    #[serde(deserialize_with = "deser_account")]
    #[serde(serialize_with = "ser_account")]
    #[serde(rename = "schoolid")]
    pub account: Account,
}
impl Default for User {
    fn default() -> Self {
        Self::new(String::new(), String::new(), BTreeMap::new())
    }
}

fn deser_account<'de, D: Deserializer<'de>>(deser: D) -> Result<Account, D::Error> {
    let schoolid = String::deserialize(deser)?;
    Ok(Account::new(schoolid, BTreeMap::new()))
}

fn ser_account<S: Serializer>(account: &Account, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(&account.schoolid)
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.userid == other.userid && self.account.schoolid == other.account.schoolid
    }
}
impl Eq for User {}
impl PartialOrd for User {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for User {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.userid
            .cmp(&other.userid)
            .then_with(|| self.account.schoolid.cmp(&self.account.schoolid))
    }
}

// basic stuff
impl User {
    /// create new instance of [`User`]
    pub fn new(userid: String, schoolid: String, rename: BTreeMap<String, String>) -> Self {
        Self {
            userid,
            account: Account::new(schoolid, rename),
        }
    }

    /// log in to a [`User`] account from cli and write it to `conf`!
    /// # Errors
    /// getting school-list, selecting school
    pub fn login(userid: String, conf: &mut Config) -> Res<Self> {
        info!("creating user ({userid}) from cli");

        let schools = schools::get()?;
        let items = schools.iter().map(|s| &s.nev).collect::<Vec<_>>();
        let school_idx = Select::new("your school's name:", items)
            .raw_prompt()?
            .index;
        let schoolid = schools[school_idx].azonosito.clone();
        info!("received schoolid {schoolid} from cli");

        let user = Self::new(userid, schoolid, BTreeMap::new());
        user.get_userinfo()?;

        user.save(conf);
        Ok(user)
    }

    /// save [`User`] credentials
    /// also set as default
    fn save(&self, conf: &mut Config) {
        conf.users.insert(self.clone());
        conf.switch_user_to(&self.userid);
    }

    /// load `who`(id or name) from `conf`
    pub fn load(conf: &Config, who: impl AsRef<str>) -> Option<Self> {
        let whose_id = conf.get_userid(who)?;
        let mut def_usr = conf.users.iter().find(|u| u.userid == whose_id).cloned()?;
        def_usr.account.rename.clone_from(&conf.rename);
        Some(def_usr)
    }
}

// interacting with API
impl User {
    /// helper fn, stores `content` of `kind` to `self.0.userid` cache-dir
    fn store_cache<S: Serialize>(&self, content: &S) -> Res<()> {
        let kind = utils::type_to_kind_name::<S>()?;

        let content = serde_json::to_string(content)?;
        cache::store(&self.userid, &kind, &content)
    }
    /// helper fn, loads cache of `kind` from `self.0.userid` cache-dir
    fn load_cache<D: for<'a> Deserialize<'a>>(&self) -> Option<(LDateTime, D)> {
        let kind = utils::type_to_kind_name::<D>().ok()?;
        if std::env::var("NO_CACHE").is_ok_and(|nc| nc == "1") && kind != "token" {
            log::info!("ignoring cache: manually triggered 'no cache' error");
            eprintln!("ignoring cache: manually triggered 'no cache' error");
            return None;
        }

        let (cache_t, content) = cache::load(&self.userid, &kind)?;
        let deserd = serde_json::from_str(&content)
            .inspect_err(|e| {
                error!("{e:?} - couldn't deserialize {kind}: {content}");
                eprintln!("error: {e:?}, check logs with `cat $(rsfilc --cache-dir)/rsfilc.log`");
            })
            .ok()?;
        Some((cache_t, deserd))
    }
    fn fetch_vec<E>(&self, query: E::Args) -> Res<Vec<E>>
    where
        E: ekreta::Endpoint + for<'a> Deserialize<'a>,
    {
        self.account.fetch_vec(query, &self.headers()?)
    }
    /// get headers which are necessary for making certain requests
    pub fn headers(&self) -> Res<HeaderMap> {
        let token = self.get_token()?.access_token;
        Ok(HeaderMap::from_iter([
            (header::AUTHORIZATION, format!("Bearer {token}").parse()?),
            (header::USER_AGENT, consts::USER_AGENT.parse()?),
        ]))
    }

    /// will read password from tty if there's no cached `Token`, otherwise uses `refresh_token`
    fn get_token(&self) -> Res<Token> {
        if let Some((cache_t, cached_token)) = self.load_cache::<Token>() {
            if Local::now().signed_duration_since(cache_t)
                < TimeDelta::seconds(cached_token.expires_in.into())
            {
                return Ok(cached_token);
            }
            // refresh token
            let token = self.account.refresh_token(&cached_token.refresh_token)?;
            self.store_cache(&token)?;
            return Ok(token);
        }
        let password = Password::new("account password:")
            .with_display_mode(PasswordDisplayMode::Masked)
            .prompt()?;
        info!("received password {} from cli", "*".repeat(password.len()));

        let token = self
            .account
            .fetch_token(password, self.userid.clone())
            .inspect_err(|e| {
                log::error!("error fetching token: {e}");
                eprintln!("error fetching token: {e}");
            })?;
        self.store_cache(&token)?;
        info!("received token");
        Ok(token)
    }
    pub fn get_userinfo(&self) -> Res<ekreta::UserInfo> {
        if let Some((_, cached_info)) = self.load_cache::<ekreta::UserInfo>()
            && cached_info.next_downtime() + TimeDelta::hours(4) > Local::now()
        {
            return Ok(cached_info);
        }
        let fetched_info = self.account.fetch_info(&self.headers()?)?;
        self.store_cache(&fetched_info)?;
        Ok(fetched_info)
    }

    gen_get_for! { get_evals, Eval, false,
        (|evals: &mut Vec<Eval>| {
            evals.sort_unstable_by_key(|e| e.keszites_datuma);
            evals.dedup_by_key(|e| e.uid.clone());
        })
    }

    pub fn get_timetable(&self, day: NaiveDate, whole_week: bool) -> Res<Vec<Lesson>> {
        let num_days_from_mon = day.weekday().number_from_monday() - 1;
        let days_from_mon = TimeDelta::days(num_days_from_mon.into());
        let days_till_sun = TimeDelta::days((7 - num_days_from_mon - 1).into());
        let from = if whole_week { day - days_from_mon } else { day };
        let to = if whole_week { day + days_till_sun } else { day };
        debug!("fetching tt, whole week: {whole_week}, from {from} to {to}");

        let (cache_t, cached_tt) = self.load_cache::<Vec<Lesson>>().unzip();
        if let Some(lessons) = cached_tt.as_ref() {
            let is_cached = |cl: &Lesson| cl.date_naive() == day;
            let fresh_cache = |ct: LDateTime| (ct - Local::now()).abs() < TimeDelta::seconds(8);
            if !whole_week && cache_t.is_some_and(fresh_cache) && lessons.iter().any(is_cached) {
                warn!("warm lesson cache hit (< 8s), using instead of refetching");
                return Ok(lessons.iter().filter(|&x| is_cached(x)).cloned().collect());
            }
        }
        let in_fetched_irval = |lsn: &Lesson| (from..=to).contains(&lsn.date_naive());
        let remain_relevant = |lessons: &mut Vec<Lesson>| lessons.retain(in_fetched_irval);
        match self.fetch_vec((from, to)) {
            Ok(mut fetched_items) => {
                let mut lessons = cached_tt.unwrap_or_default();
                // delete cached if fresh was fetched for that period
                lessons.retain(|cl| !in_fetched_irval(cl));
                lessons.append(&mut fetched_items);
                lessons.sort_unstable_by_key(|l| l.kezdet_idopont);
                self.store_cache(&lessons)?;
                remain_relevant(&mut lessons);
                Ok(lessons)
            }
            Err(e) => {
                error!("only loading cached lessons, couldn't reach E-Kréta server: {e:?}");
                eprintln!("only loading cached lessons, couldn't reach E-Kréta server: {e:?}");
                let mut lessons = cached_tt.ok_or("nothing cached")?;
                remain_relevant(&mut lessons);
                // shouldn't have any lesson on weekends by default
                if lessons.is_empty() && days_till_sun > TimeDelta::days(1) {
                    Err("nothing cached for this period".into())
                } else {
                    Ok(lessons)
                }
            }
        }
    }

    gen_get_for! { get_tests, Ancd, false,
        (|tests: &mut Vec<Ancd>| {
            tests.sort_unstable_by_key(|a| (a.datum, a.uid.clone()));
            tests.dedup_by_key(|a| a.uid.clone());
        })
    }

    gen_get_for! { get_absences, Absence, true,
        (|absences: &mut Vec<Absence>| {
            absences.sort_unstable_by_key(|a| (a.ora.kezdo_datum, !a.igazolt()));
            absences.dedup_by_key(|a| a.ora.clone());
        })
    }
}

/// [`Msg`]s and [`Attachment`]s
impl User {
    /// Download all [`Attachment`]s of this [`Msg`] to [`download_dir()`].
    ///
    /// # Errors
    /// - net
    pub fn download_attachments(&self, msg: &MsgItem) -> Res<()> {
        for am in &msg.uzenet.csatolmanyok {
            let download_to = messages::download_attachment_to(am);
            info!("downloading file://{}", download_to.display());
            // don't download if already exists
            if download_to.exists() {
                debug!("not downloading, already done");
                continue;
            }
            self.account
                .download_attachment_to(am.azonosito, download_to, &self.headers()?)?;

            info!("received file {}", &am.fajl_nev);
        }
        Ok(())
    }

    /// Fetch [`Msg`]s between `from` and `to`.
    /// Also download all `[Attachment]`s each [`Msg`] has.
    /// # Errors
    /// - net
    pub fn get_msg(&self, oview: &MsgOview) -> Res<MsgItem> {
        let (_, cached_msgs) = self.load_cache::<Vec<MsgItem>>().unzip();
        let mut cached_msgs = cached_msgs.unwrap_or_default();

        if let Some(cache_hit) = cached_msgs.iter().find(|j| j.azonosito == oview.azonosito) {
            return Ok(cache_hit.clone());
        }
        let fetched_msg = self.account.fetch_full_msg(Some(oview), &self.headers()?)?;

        cached_msgs.push(fetched_msg.clone());
        cached_msgs.sort_unstable_by_key(|m| m.uzenet.kuldes_datum);
        cached_msgs.dedup_by_key(|m| m.azonosito);
        self.store_cache(&cached_msgs)?;
        self.download_all_attachments(&fetched_msg)?;

        Ok(fetched_msg)
    }

    fn download_all_attachments(&self, msg: &MsgItem) -> Res<()> {
        self.download_attachments(msg)
            .inspect_err(|e| error!("couldn't fetch from E-Kréta server: {e:?}"))
    }

    pub fn get_msg_oviews(&self) -> Res<Vec<MsgOview>> {
        fn inner(usr: &User) -> Res<Vec<MsgOview>> {
            usr.account.fetch_msg_oviews(&usr.headers()?)
        } // used as a catcher of the `?` in the `usr.headers()?`, not to return too early on a `NO_NET=1`
        match inner(self) {
            Ok(mut msg_oviews) => {
                msg_oviews.sort_unstable_by_key(|a| a.uzenet_kuldes_datum);
                if !msg_oviews.is_empty() {
                    self.store_cache(&msg_oviews)?;
                }
                Ok(msg_oviews)
            }
            Err(e) => {
                error!("couldn't reach E-Kréta server: {e:?}, only loading cached messages");
                eprintln!("couldn't reach E-Kréta server: {e:?}, only loading cached messages");
                let (_t, cached_msg_oviews) = self.load_cache().ok_or("nothing cached")?;
                Ok(cached_msg_oviews)
            }
        }
    }

    gen_get_for! { get_note_msgs, ekreta::NoteMsg, false,
        (|nmsgs: &mut Vec<ekreta::NoteMsg>| {
            nmsgs.sort_unstable_by_key(|nmsg| nmsg.datum);
            nmsgs.dedup();
        })
    }

    /// load data from cache, fetch remaining(or full, depending on `fix_irval`) interval, merge these two sources
    /// # NOTE
    /// - if any of the two fails, it will be logged, but ignored and the other source will be used instead
    /// - don't forget to deduplicate the returned Vec **properly**
    fn load_n_fetch<Ep>(&self, mut irval: OptIrval, fix_irval: bool) -> Res<Vec<Ep>>
    where
        Ep: ekreta::Endpoint<Args = OptIrval> + for<'a> Deserialize<'a> + Clone,
    {
        let (cache_t, cached) = self.load_cache::<Vec<Ep>>().unzip();
        let orig_irval = irval;

        if fix_irval && cached.is_some() {
            irval = utils::fix_from(cache_t, irval);
        }

        let fetched = self.fetch_vec::<Ep>(irval);

        let mut items = match fetched {
            Ok(fetched_items) => {
                let mut cached = cached.unwrap_or_default();
                let not_fetched =
                    |dt: LDateTime| irval.0.is_none_or(|from| dt.date_naive() <= from);
                let orig_len = cached.len();
                cached.retain(|item| item.when().is_none_or(not_fetched));
                let overwritten = orig_len - cached.len();
                log::info!("{overwritten} cached items were overwritten with fetched");
                Ok([cached, fetched_items].concat())
            }
            Err(e) => {
                let kind_name = utils::type_to_kind_name::<Ep>().unwrap_or_default();
                error!("only loading cached {kind_name}, couldn't reach E-Kréta server: {e:?}");
                eprintln!("only loading cached {kind_name}, couldn't reach E-Kréta server: {e:?}");
                cached.ok_or("nothing cached".to_owned())
            }
        }?;
        let orig_len = items.len();
        let in_irval = |dt: LDateTime| {
            orig_irval.0.is_none_or(|from| from <= dt.date_naive())
                && orig_irval.1.is_none_or(|to| dt.date_naive() <= to)
        };
        items.retain(|item| item.when().is_none_or(in_irval));
        let deleted = orig_len - items.len();
        log::info!("deleted {deleted} items that weren't in interval asked");
        Ok(items)
    }
}
