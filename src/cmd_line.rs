use gtk;
use gtk::prelude::*;

//TODO: levels
pub struct CmdLine {
    popover: gtk::Popover,
}

impl CmdLine {
    pub fn new(drawing: &gtk::DrawingArea) -> Self {
        let popover = gtk::Popover::new(Some(drawing));
        popover.set_modal(false);
        let edit_frame = gtk::Frame::new(None);
        edit_frame.set_shadow_type(gtk::ShadowType::In);
        let drawing_area = gtk::DrawingArea::new();
        edit_frame.add(&drawing_area);
        edit_frame.show_all();

        popover.add(&edit_frame);

        CmdLine {
            popover,
        }
    }

    pub fn show(&self) {
        self.popover.popup();
    }
}
