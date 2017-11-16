
mod client;
mod handler;
mod mode_info;
mod redraw_handler;
mod repaint_mode;

pub use self::redraw_handler::{RedrawEvents, GuiApi};
pub use self::repaint_mode::RepaintMode;
pub use self::client::{NeovimClient, NeovimClientAsync, NeovimRef};
pub use self::mode_info::{ModeInfo, CursorShape};

use std::error;
use std::fmt;
use std::env;
use std::process::{Stdio, Command};
use std::result;
use std::sync::Arc;

use neovim_lib::{Neovim, NeovimApi, Session, UiAttachOptions, CallError};

use ui::UiMutex;
use shell;
use nvim_config::NvimConfig;

#[derive(Debug)]
pub struct NvimInitError {
    source: Box<error::Error>,
    cmd: Option<String>,
}

impl NvimInitError {
    pub fn new_post_init<E>(error: E) -> NvimInitError
    where
        E: Into<Box<error::Error>>,
    {
        NvimInitError {
            cmd: None,
            source: error.into(),
        }
    }

    pub fn new<E>(cmd: &Command, error: E) -> NvimInitError
    where
        E: Into<Box<error::Error>>,
    {
        NvimInitError {
            cmd: Some(format!("{:?}", cmd)),
            source: error.into(),
        }
    }

    pub fn source(&self) -> String {
        format!("{}", self.source)
    }

    pub fn cmd(&self) -> Option<&String> {
        self.cmd.as_ref()
    }
}

impl fmt::Display for NvimInitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.source)
    }
}

impl error::Error for NvimInitError {
    fn description(&self) -> &str {
        "Can't start nvim instance"
    }

    fn cause(&self) -> Option<&error::Error> {
        Some(&*self.source)
    }
}

pub fn start(
    shell: Arc<UiMutex<shell::State>>,
    nvim_bin_path: Option<&String>,
) -> result::Result<Neovim, NvimInitError> {
    let mut cmd = if let Some(path) = nvim_bin_path {
        Command::new(path)
    } else {
        Command::new("nvim")
    };

    cmd.arg("--embed")
        .arg("--headless")
        // Swap files are disabled because it shows message window on start up but frontend can't detect it.
        .arg("-n")
        .arg("--cmd")
        .arg("set termguicolors")
        .arg("--cmd")
        .arg("let g:GtkGuiLoaded = 1")
        .stderr(Stdio::inherit());

    if let Ok(runtime_path) = env::var("NVIM_GTK_RUNTIME_PATH") {
        cmd.arg("--cmd").arg(
            format!("let &rtp.=',{}'", runtime_path),
        );
    } else if let Some(prefix) = option_env!("PREFIX") {
        cmd.arg("--cmd").arg(format!(
            "let &rtp.=',{}/share/nvim-gtk/runtime'",
            prefix
        ));
    } else {
        cmd.arg("--cmd").arg("let &rtp.=',runtime'");
    }

    if let Some(nvim_config) = NvimConfig::config_path() {
        if let Some(path) = nvim_config.to_str() {
            cmd.arg("--cmd").arg(format!("source {}", path));
        }
    }

    let session = Session::new_child_cmd(&mut cmd);

    let session = match session {
        Err(e) => return Err(NvimInitError::new(&cmd, e)),
        Ok(s) => s,
    };

    let mut nvim = Neovim::new(session);

    nvim.session.start_event_loop_handler(
        handler::NvimHandler::new(shell),
    );

    Ok(nvim)
}

pub fn post_start_init(
    nvim: NeovimClientAsync,
    open_path: Option<&String>,
    cols: u64,
    rows: u64,
) -> result::Result<(), NvimInitError> {
    let mut opts = UiAttachOptions::new();
    opts.set_popupmenu_external(true);
    opts.set_tabline_external(true);
    nvim.borrow().unwrap().ui_attach(cols, rows, &opts).map_err(
        NvimInitError::new_post_init,
    )?;
    nvim.borrow().unwrap().command("runtime! ginit.vim").map_err(
        NvimInitError::new_post_init,
    )?;

    if let Some(path) = open_path {
        nvim.borrow().unwrap().command(&format!("e {}", path)).map_err(
            NvimInitError::new_post_init,
        )?;
    }

    Ok(())
}


pub trait ErrorReport<T> {
    fn report_err(&self, nvim: &mut NeovimApi);

     fn ok_and_report(self, nvim: &mut NeovimApi) -> Option<T>;
}

impl<T> ErrorReport<T> for result::Result<T, CallError> {
    fn report_err(&self, _: &mut NeovimApi) {
        if let Err(ref err) = *self {
            println!("{}", err);
            //nvim.report_error(&err_msg).expect("Error report error :)");
        }
    }

    fn ok_and_report(self, nvim: &mut NeovimApi) -> Option<T> {
        self.report_err(nvim);
        self.ok()
    }
}

