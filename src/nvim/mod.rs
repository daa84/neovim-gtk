
mod client;
mod handler;
mod mode_info;
mod redraw_handler;
mod repaint_mode;
mod ext;

pub use self::redraw_handler::{RedrawEvents, GuiApi, CompleteItem};
pub use self::repaint_mode::RepaintMode;
pub use self::client::{NeovimClient, NeovimClientAsync, NeovimRef};
pub use self::mode_info::{ModeInfo, CursorShape};
pub use self::ext::ErrorReport;

use std::error;
use std::fmt;
use std::env;
use std::process::{Stdio, Command};
use std::result;
use std::sync::Arc;
use std::time::Duration;

use neovim_lib::{Neovim, NeovimApi, Session, UiAttachOptions};

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
    timeout: Option<Duration>,
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

    let mut session = match session {
        Err(e) => return Err(NvimInitError::new(&cmd, e)),
        Ok(s) => s,
    };

    session.set_timeout(timeout.unwrap_or(Duration::from_millis(10_000)));

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
    nvim.borrow()
        .unwrap()
        .ui_attach(
            cols,
            rows,
            UiAttachOptions::new()
                .set_popupmenu_external(true)
                .set_tabline_external(true)
                .set_cmdline_external(true),
        )
        .map_err(NvimInitError::new_post_init)?;

    nvim.borrow()
        .unwrap()
        .command("runtime! ginit.vim")
        .map_err(NvimInitError::new_post_init)?;

    if let Some(path) = open_path {
        nvim.borrow()
            .unwrap()
            .command(&format!("e {}", path))
            .map_err(NvimInitError::new_post_init)?;
    }

    Ok(())
}

