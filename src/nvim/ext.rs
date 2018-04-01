use std::result;

use neovim_lib::CallError;

pub trait ErrorReport<T> {
    fn report_err(&self);

    fn ok_and_report(self) -> Option<T>;
}

impl<T> ErrorReport<T> for result::Result<T, CallError> {
    fn report_err(&self) {
        if let Err(ref err) = *self {
            error!("{}", err);
        }
    }

    fn ok_and_report(self) -> Option<T> {
        self.report_err();
        self.ok()
    }
}
