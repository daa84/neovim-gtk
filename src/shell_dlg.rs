use ui::{SH, Ui};
use neovim_lib::{NeovimApi, CallError, Value};
use gtk;
use gtk::prelude::DialogExtManual;
use gtk::{DialogExt, MessageDialog, MessageType, ButtonsType};

pub fn can_close_window(ui: &Ui) -> bool {
    match get_changed_buffers() {
        Ok(vec) => {
            if !vec.is_empty() {
                show_not_saved_dlg(ui, &vec)
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

fn show_not_saved_dlg(ui: &Ui, changed_bufs: &Vec<String>) -> bool {
    let mut changed_files = changed_bufs.iter()
        .map(|n| if n.is_empty() { "<No name>" } else { n })
        .fold(String::new(), |acc, v| acc + v + "\n");
    changed_files.pop();

    let flags = gtk::DIALOG_MODAL | gtk::DIALOG_DESTROY_WITH_PARENT;
    let dlg = MessageDialog::new(ui.window.as_ref(),
                                 flags,
                                 MessageType::Question,
                                 ButtonsType::None,
                                 &format!("Save changes to '{}'?", changed_files));

    const ACCEPT_ID: i32 = 1;
    const CLOSE_ID: i32 = 2;
    const REJECT_ID: i32 = 3;

    dlg.add_buttons(&[("_Yes", ACCEPT_ID), ("_No", CLOSE_ID), ("_Cancel", REJECT_ID)]);

    match dlg.run() {
        ACCEPT_ID => true,
        CLOSE_ID => true,
        REJECT_ID => false,
        _ => false,
    }
}

fn get_changed_buffers() -> Result<Vec<String>, CallError> {
    SHELL!(shell = {
        let mut nvim = shell.nvim();
        let buffers = nvim.get_buffers().unwrap();

        Ok(buffers.iter()
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
    })
}
