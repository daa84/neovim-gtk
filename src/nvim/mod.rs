mod client;
mod handler;
mod redraw_handler;
mod repaint_mode;
mod ext;

pub use self::redraw_handler::{CompleteItem, NvimCommand};
pub use self::repaint_mode::RepaintMode;
pub use self::client::{NeovimClient, NeovimClientAsync, NeovimRef};
pub use self::ext::ErrorReport;
pub use self::handler::NvimHandler;

use std::error;
use std::fmt;
use std::env;
use std::process::{Command, Stdio};
use std::result;
use std::time::Duration;

use neovim_lib::{Neovim, NeovimApi, NeovimApiAsync, Session, UiAttachOptions};

use misc::escape_filename;
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

#[cfg(target_os = "windows")]
fn set_windows_creation_flags(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
}

pub fn start(
    handler: NvimHandler,
    nvim_bin_path: Option<&String>,
    timeout: Option<Duration>,
    args_for_neovim: Vec<String>,
) -> result::Result<Neovim, NvimInitError> {
    let mut cmd = if let Some(path) = nvim_bin_path {
        Command::new(path)
    } else {
        Command::new("nvim")
    };

    cmd.arg("--embed")
        .arg("--cmd")
        .arg("set termguicolors")
        .arg("--cmd")
        .arg("let g:GtkGuiLoaded = 1")
        .stderr(Stdio::inherit());

    #[cfg(target_os = "windows")]
    set_windows_creation_flags(&mut cmd);

    if let Ok(runtime_path) = env::var("NVIM_GTK_RUNTIME_PATH") {
        cmd.arg("--cmd")
            .arg(format!("let &rtp.=',{}'", runtime_path));
    } else if let Some(prefix) = option_env!("PREFIX") {
        cmd.arg("--cmd")
            .arg(format!("let &rtp.=',{}/share/nvim-gtk/runtime'", prefix));
    } else {
        cmd.arg("--cmd").arg("let &rtp.=',runtime'");
    }

    if let Some(nvim_config) = NvimConfig::config_path() {
        if let Some(path) = nvim_config.to_str() {
            cmd.arg("--cmd").arg(format!("source {}", path));
        }
    }

    for arg in args_for_neovim {
        cmd.arg(arg);
    }

    let session = Session::new_child_cmd(&mut cmd);

    let mut session = match session {
        Err(e) => return Err(NvimInitError::new(&cmd, e)),
        Ok(s) => s,
    };

    session.set_timeout(timeout.unwrap_or(Duration::from_millis(10_000)));

    let mut nvim = Neovim::new(session);

    nvim.session.start_event_loop_handler(handler);

    Ok(nvim)
}

pub fn post_start_init(
    nvim: NeovimClientAsync,
    open_paths: Vec<String>,
    cols: i64,
    rows: i64,
    input_data: Option<String>,
) -> result::Result<(), NvimInitError> {
    nvim.borrow()
        .unwrap()
        .ui_attach(
            cols,
            rows,
            UiAttachOptions::new()
                .set_popupmenu_external(true)
                .set_tabline_external(true)
                .set_linegrid_external(true)
                .set_hlstate_external(true)
        )
        .map_err(NvimInitError::new_post_init)?;

    nvim.borrow()
        .unwrap()
        .command("runtime! ginit.vim")
        .map_err(NvimInitError::new_post_init)?;

    if !open_paths.is_empty() {
        let command = open_paths
            .iter()
            .fold(":ar".to_owned(), |command, filename| {
                let filename = escape_filename(filename);
                command + " " + &filename
            });
        nvim.borrow()
            .unwrap()
            .command_async(&command)
            .cb(|r| r.report_err())
            .call();
    } else {
        if let Some(input_data) = input_data {
            let mut nvim = nvim.borrow().unwrap();
            let buf = nvim.get_current_buf().ok_and_report();

            if let Some(buf) = buf {
                buf.set_lines(
                    &mut *nvim,
                    0,
                    0,
                    true,
                    input_data.lines().map(|l| l.to_owned()).collect(),
                ).report_err();
            }
        }
    }

    Ok(())
}
