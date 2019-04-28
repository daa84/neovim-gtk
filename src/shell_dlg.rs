use std::cell::RefCell;

use glib::translate::FromGlib;
use gtk;
use gtk::prelude::*;
use gtk::{ButtonsType, MessageDialog, MessageType};

use neovim_lib::{CallError, NeovimApi, Value};
use crate::shell::Shell;
use crate::ui::{Components, UiMutex};

pub fn can_close_window(comps: &UiMutex<Components>, shell: &RefCell<Shell>) -> bool {
    let shell = shell.borrow();
    match get_changed_buffers(&*shell) {
        Ok(vec) => {
            if !vec.is_empty() {
                show_not_saved_dlg(comps, &*shell, &vec)
            } else {
                true
            }
        }
        Err(ref err) => {
            error!("Error getting info from nvim: {}", err);
            true
        }
    }
}

fn show_not_saved_dlg(comps: &UiMutex<Components>, shell: &Shell, changed_bufs: &[String]) -> bool {
    let mut changed_files = changed_bufs
        .iter()
        .map(|n| if n.is_empty() { "<No name>" } else { n })
        .fold(String::new(), |acc, v| acc + v + "\n");
    changed_files.pop();

    let flags = gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT;
    let dlg = MessageDialog::new(
        Some(comps.borrow().window()),
        flags,
        MessageType::Question,
        ButtonsType::None,
        &format!("Save changes to '{}'?", changed_files),
    );

    dlg.add_buttons(&[
        ("_Yes", gtk::ResponseType::Yes),
        ("_No", gtk::ResponseType::No),
        ("_Cancel", gtk::ResponseType::Cancel),
    ]);

    let res = match gtk::ResponseType::from_glib(dlg.run()) {
        gtk::ResponseType::Yes => {
            let state = shell.state.borrow();
            let mut nvim = state.nvim().unwrap();
            match nvim.command("wa") {
                Err(ref err) => {
                    error!("Error: {}", err);
                    false
                }
                _ => true,
            }
        }
        gtk::ResponseType::No => true,
        gtk::ResponseType::Cancel | _ => false,
    };

    dlg.destroy();

    res
}

fn get_changed_buffers(shell: &Shell) -> Result<Vec<String>, CallError> {
    let state = shell.state.borrow();
    let nvim = state.nvim();
    if let Some(mut nvim) = nvim {
        let buffers = nvim.list_bufs().unwrap();

        Ok(buffers
            .iter()
            .map(|buf| {
                (
                    match buf.get_option(&mut nvim, "modified") {
                        Ok(Value::Boolean(val)) => val,
                        Ok(_) => {
                            warn!("Value must be boolean");
                            false
                        }
                        Err(ref err) => {
                            error!("Something going wrong while getting buffer option: {}", err);
                            false
                        }
                    },
                    match buf.get_name(&mut nvim) {
                        Ok(name) => name,
                        Err(ref err) => {
                            error!("Something going wrong while getting buffer name: {}", err);
                            "<Error>".to_owned()
                        }
                    },
                )
            })
            .filter(|e| e.0)
            .map(|e| e.1)
            .collect())
    } else {
        Ok(vec![])
    }
}
