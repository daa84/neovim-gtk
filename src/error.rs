use std::ops::Deref;

use htmlescape::encode_minimal;

use gtk;
use gtk::prelude::*;

use crate::shell;

pub struct ErrorArea {
    base: gtk::Box,
    label: gtk::Label,
}

impl ErrorArea {
    pub fn new() -> Self {
        let base = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        let label = gtk::Label::new(None);
        label.set_line_wrap(true);
        let error_image = gtk::Image::new_from_icon_name(Some("dialog-error"), gtk::IconSize::Dialog);
        base.pack_start(&error_image, false, true, 10);
        base.pack_start(&label, true, true, 1);

        ErrorArea { base, label }
    }

    pub fn show_nvim_init_error(&self, err: &str) {
        error!("Can't initialize nvim: {}", err);
        self.label.set_markup(&format!(
            "<big>Can't initialize nvim:</big>\n\
             <span foreground=\"red\"><i>{}</i></span>\n\n\
             <big>Possible error reasons:</big>\n\
             &#9679; Not supported nvim version (minimum supported version is <b>{}</b>)\n\
             &#9679; Error in configuration file (init.vim or ginit.vim)",
            encode_minimal(err),
            shell::MINIMUM_SUPPORTED_NVIM_VERSION
        ));
        self.base.show_all();
    }

    pub fn show_nvim_start_error(&self, err: &str, cmd: &str) {
        error!("Can't start nvim: {}\nCommand line: {}", err, cmd);
        self.label.set_markup(&format!(
            "<big>Can't start nvim instance:</big>\n\
             <i>{}</i>\n\
             <span foreground=\"red\"><i>{}</i></span>\n\n\
             <big>Possible error reasons:</big>\n\
             &#9679; Not supported nvim version (minimum supported version is <b>{}</b>)\n\
             &#9679; Error in configuration file (init.vim or ginit.vim)\n\
             &#9679; Wrong nvim binary path \
             (right path can be passed with <i>--nvim-bin-path=path_here</i>)",
            encode_minimal(cmd),
            encode_minimal(err),
            shell::MINIMUM_SUPPORTED_NVIM_VERSION
        ));
        self.base.show_all();
    }
}

impl Deref for ErrorArea {
    type Target = gtk::Box;

    fn deref(&self) -> &gtk::Box {
        &self.base
    }
}
