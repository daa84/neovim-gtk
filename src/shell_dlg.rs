use ui::{SH, Ui};
use neovim_lib::{NeovimApi, CallError, Value};
use gtk;
use gtk_sys;
use gtk::{Dialog, DialogExt};

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
    let flags = gtk::DIALOG_MODAL | gtk::DIALOG_DESTROY_WITH_PARENT;
    let dlg = Dialog::new_with_buttons(Some("Question"),
                                       ui.window.as_ref(),
                                       flags,
                                       &[("_OK", gtk_sys::GTK_RESPONSE_ACCEPT as i32),
                                         ("_Cancel", gtk_sys::GTK_RESPONSE_REJECT as i32)]);

    dlg.run();

    true
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
