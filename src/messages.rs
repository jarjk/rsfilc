//! messages from teachers and staff

use crate::{config::CONFIG, paths::download_dir, time::MyDate, user::User, utils};
use ekreta::{Endpoint, Res};
use inquire::Select;
use std::{char, fmt::Write};

pub fn handle_note_msgs(user: &User, mut id: Option<isize>, interactive: bool, args: &crate::Args) -> Res<()> {
    let notes = user.get_note_msgs((None, None))?;
    if interactive {
        let items = notes
            .iter()
            .map(|t| {
                format!(
                    "[{}] {}: {}",
                    &t.datum.date_naive(),
                    &t.keszito_tanar_neve,
                    &t.cim
                )
            })
            .rev()
            .collect::<Vec<_>>();
        match isize::try_from(Select::new("Open message:", items).with_vim_mode(CONFIG.get_vim_mode()).raw_prompt()?.index).ok() {
            Some(signed_id) => id = Option::from(isize::try_from(notes.len())? - signed_id - 1),
            _ => {}
        };
        // info!("received messageid {id} from cli");
    }
    if let Some(ix) = id_to_ix(id, notes.len()) {
        let Some(nm) = notes.get(ix) else {
            return Err(format!("can't find message with id: {ix}").into());
        };
        let print = if args.machine {
            serde_json::to_string(nm)?
        } else {
            disp_nm(nm)
        };
        println!("{print}");
        return Ok(());
    }

    let data = notes.iter().enumerate().collect::<Vec<_>>();
    let headers = ["ID", "TÁRGYA", "TŐLE", "EKKOR"].iter();
    #[rustfmt::skip]
    let disp = if args.machine { None } else { Some(preview_nm) };
    utils::print_table(&data, headers, args.reverse, args.number, disp)
}

pub fn handle(user: &User, mut id: Option<isize>, interactive: bool, args: &crate::Args) -> Res<()> {
    let msg_oviews = user.get_msg_oviews()?;
    if interactive {
        let items = msg_oviews
            .iter()
            .map(|t| {
                format!(
                    "[{}] {}{}",
                    &t.uzenet_kuldes_datum.date(),
                    t.uzenet_felado_nev
                        .as_ref()
                        .map(|n| format!("{n}: "))
                        .unwrap_or_default(),
                    &t.uzenet_targy
                )
            })
            .rev()
            .collect::<Vec<_>>();
        match isize::try_from(Select::new("Open message:", items).with_vim_mode(CONFIG.get_vim_mode()).raw_prompt()?.index).ok() {
            Some(signed_id) => id = Option::from(isize::try_from(msg_oviews.len())? - signed_id - 1),
            _ => {}
        };
        // info!("received messageid {id} from cli");
    }
    if let Some(ix) = id_to_ix(id, msg_oviews.len()) {
        let msg_oview = msg_oviews
            .get(ix)
            .ok_or(format!("can't find message with id: {ix}"))?;
        let msg = user.get_msg(msg_oview)?;
        let print = if args.machine {
            serde_json::to_string(&msg)?
        } else {
            disp_msg(&msg)
        };
        println!("{print}");
        return Ok(());
    }

    let data = msg_oviews.iter().enumerate().collect::<Vec<_>>();
    let headers = ["ID", "TÁRGYA", "TŐLE", "EKKOR", "CSATOLMÁNYA"].iter();
    #[rustfmt::skip]
    let disp = if args.machine { None } else { Some(disp_oviews) };
    utils::print_table(&data, headers, args.reverse, args.number, disp)
}

/// make a `-id` reverse of id, eg len: 8, id: -1 -> ix = 7
fn id_to_ix(id: Option<isize>, len: usize) -> Option<usize> {
    let id = id?;
    if id >= 0 {
        return usize::try_from(id).ok();
    }
    let len = isize::try_from(len).ok()?;
    usize::try_from(len + id).ok()
}

fn disp_oviews(preview: &(usize, &ekreta::MsgOview)) -> Vec<String> {
    let msg = preview.1;
    let datetime = msg.when().unwrap().pretty();
    let subj = msg.uzenet_targy.clone();
    let prefix = msg.uzenet_felado_nev.clone().unwrap_or_default();
    let name = msg.uzenet_felado_titulus.clone().unwrap_or_default();
    let sender = format!("{prefix} {name}");
    // if !msg_oview.is_elolvasva {
    //     writeln!(f, "Olvasatlan")?;
    // }
    let n = preview.0.to_string();
    let mut row = vec![n, subj, sender, datetime];
    if msg.has_csatolmany {
        row.push(String::from("van"));
    }
    row
}

pub fn download_attachment_to(am: &ekreta::Attachment) -> std::path::PathBuf {
    download_dir().join(am.fajl_nev.replace(char::is_whitespace, "_"))
}

pub fn disp_msg(msg: &ekreta::MsgItem) -> String {
    let mut f = String::new();
    _ = writeln!(&mut f, "| Tárgy: {}", msg.uzenet.targy);
    for am in &msg.uzenet.csatolmanyok {
        let out_path = download_attachment_to(am);
        _ = writeln!(&mut f, "| Csatolmány: \"file://{}\"", out_path.display());
    }
    let name = &msg.tipus.nev;
    _ = writeln!(&mut f, "| {name}: {}", msg.when().unwrap().pretty());
    let sender = &msg.uzenet.felado_nev;
    _ = writeln!(&mut f, "| Feladó: {sender} {}", msg.uzenet.felado_titulus);
    let rendered = nanohtml2text::html2text(&msg.uzenet.szoveg);
    _ = write!(&mut f, "\n{rendered}");
    // if !msg.is_elolvasva {
    //  writeln!(f, "Olvasatlan")?;
    // }
    f
}

pub fn preview_nm(preview: &(usize, &ekreta::NoteMsg)) -> Vec<String> {
    let note_msg = preview.1;
    let n = preview.0.to_string();
    let subj = note_msg.cim.clone();
    let datetime = note_msg.datum.pretty();
    let sender = note_msg.keszito_tanar_neve.clone();

    vec![n, subj, sender, datetime]
}

pub fn disp_nm(note_msg: &ekreta::NoteMsg) -> String {
    let mut f = String::new();
    _ = writeln!(&mut f, "| {}", note_msg.cim);
    _ = writeln!(&mut f, "| Időpont: {}", note_msg.datum.pretty());
    _ = writeln!(&mut f, "| {}", note_msg.keszito_tanar_neve);
    let rendered = nanohtml2text::html2text(&note_msg.tartalom_formazott);
    _ = write!(&mut f, "\n{rendered}");
    f
}
