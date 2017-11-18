use gtk;
use gtk::prelude::*;

pub struct CmdLine {
    dlg: gtk::Dialog,
}

impl CmdLine {
    pub fn new() -> Self {
        let dlg = gtk::Dialog::new();
        dlg.set_modal(true);
        dlg.set_destroy_with_parent(true);

        CmdLine {
            dlg,
        }
    }

    pub fn show<W: gtk::IsA<gtk::Window>>(&self, parent: &W) {
        self.dlg.set_transient_for(parent);
        self.dlg.show();
    }
}
