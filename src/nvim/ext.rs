
use std::result;
use super::client::NeovimRef;

use neovim_lib::{NeovimApi, CallError};

pub trait ErrorReport<T> {
    fn report_err(&self, nvim: &mut NeovimApi);

    fn ok_and_report(self, nvim: &mut NeovimApi) -> Option<T>;
}

impl<T> ErrorReport<T> for result::Result<T, CallError> {
    fn report_err(&self, _: &mut NeovimApi) {
        if let Err(ref err) = *self {
            error!("{}", err);
            //nvim.report_error(&err_msg).expect("Error report error :)");
        }
    }

    fn ok_and_report(self, nvim: &mut NeovimApi) -> Option<T> {
        self.report_err(nvim);
        self.ok()
    }
}

pub trait NeovimExt: Sized {
    fn non_blocked(self) -> Option<Self>;
}

impl <'a>NeovimExt for NeovimRef<'a> {
    fn non_blocked(mut self) -> Option<Self> {
        self.get_mode().ok_and_report(&mut *self).and_then(|mode| {
            mode.iter()
                .find(|kv| {
                    kv.0.as_str().map(|key| key == "blocking").unwrap_or(false)
                })
                .map(|kv| kv.1.as_bool().unwrap_or(false))
                .and_then(|block| if block { None } else { Some(self) })
        })
    }
}
