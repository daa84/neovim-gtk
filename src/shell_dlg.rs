use std::cell::RefCell;

use ui::{Components, UiMutex};
use shell::Shell;
use neovim_lib::{NeovimApi, CallError, Value};
use gtk;
use gtk::prelude::*;
use gtk::{MessageDialog, MessageType, ButtonsType};

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
            println!("Error getting info from nvim: {}", err);
            true
        }
    }
}

fn show_not_saved_dlg(comps: &UiMutex<Components>,
                      shell: &Shell,
                      changed_bufs: &Vec<String>)
                      -> bool {
    let mut changed_files = changed_bufs
        .iter()
        .map(|n| if n.is_empty() { "<No name>" } else { n })
        .fold(String::new(), |acc, v| acc + v + "\n");
    changed_files.pop();

    let flags = gtk::DIALOG_MODAL | gtk::DIALOG_DESTROY_WITH_PARENT;
    let dlg = MessageDialog::new(Some(comps.borrow().window()),
                                 flags,
                                 MessageType::Question,
                                 ButtonsType::None,
                                 &format!("Save changes to '{}'?", changed_files));

    const SAVE_ID: i32 = 0;
    const CLOSE_WITHOUT_SAVE: i32 = 1;
    const CANCEL_ID: i32 = 2;

    dlg.add_buttons(&[("_Yes", SAVE_ID),
                      ("_No", CLOSE_WITHOUT_SAVE),
                      ("_Cancel", CANCEL_ID)]);

    let res = match dlg.run() {
        SAVE_ID => {
            let mut nvim = shell.nvim();
            match nvim.command("wa") {
                Err(ref err) => {
                    println!("Error: {}", err);
                    false
                }
                _ => true,
            }
        }
        CLOSE_WITHOUT_SAVE => true,
        CANCEL_ID => false,
        _ => false,
    };

    dlg.destroy();

    res
}

fn get_changed_buffers(shell: &Shell) -> Result<Vec<String>, CallError> {
    let mut nvim = shell.nvim();
    let buffers = nvim.get_buffers().unwrap();

    Ok(buffers
       .iter()
       .map(|buf| {
           (match buf.get_option(&mut nvim, "modified") {
               Ok(Value::Boolean(val)) => val,
               Ok(_) => {
                   println!("Value must be boolean");
                   false
               }
               Err(ref err) => {
                   println!("Something going wrong while getting buffer option: {}", err);
                   false
               }
           },
           match buf.get_name(&mut nvim) {
               Ok(name) => name,
               Err(ref err) => {
                   println!("Something going wrong while getting buffer name: {}", err);
                   "<Error>".to_owned()
               }
           })
       })
        .filter(|e| e.0)
        .map(|e| e.1)
        .collect())
}
