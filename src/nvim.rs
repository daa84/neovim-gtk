use std::error;
use std::fmt;
use std::env;
use std::process::{Stdio, Command};
use std::result;
use std::sync::Arc;
use std::ops::{Deref, DerefMut};
use std::collections::HashMap;

use neovim_lib::{Handler, Neovim, NeovimApi, Session, Value, UiAttachOptions, CallError, UiOption};
use neovim_lib::neovim_api::Tabpage;

use ui::UiMutex;
use ui_model::{ModelRect, ModelRectVec};
use shell;
use glib;

pub trait RedrawEvents {
    fn on_cursor_goto(&mut self, row: u64, col: u64) -> RepaintMode;

    fn on_put(&mut self, text: &str) -> RepaintMode;

    fn on_clear(&mut self) -> RepaintMode;

    fn on_resize(&mut self, columns: u64, rows: u64) -> RepaintMode;

    fn on_redraw(&self, mode: &RepaintMode);

    fn on_highlight_set(&mut self, attrs: &Vec<(Value, Value)>) -> RepaintMode;

    fn on_eol_clear(&mut self) -> RepaintMode;

    fn on_set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64) -> RepaintMode;

    fn on_scroll(&mut self, count: i64) -> RepaintMode;

    fn on_update_bg(&mut self, bg: i64) -> RepaintMode;

    fn on_update_fg(&mut self, fg: i64) -> RepaintMode;

    fn on_update_sp(&mut self, sp: i64) -> RepaintMode;

    fn on_mode_change(&mut self, mode: &str) -> RepaintMode;

    fn on_mouse(&mut self, on: bool) -> RepaintMode;

    fn on_busy(&mut self, busy: bool) -> RepaintMode;

    fn popupmenu_show(&mut self,
                      menu: &Vec<Vec<&str>>,
                      selected: i64,
                      row: u64,
                      col: u64)
                      -> RepaintMode;

    fn popupmenu_hide(&mut self) -> RepaintMode;

    fn popupmenu_select(&mut self, selected: i64) -> RepaintMode;

    fn tabline_update(&mut self,
                      selected: Tabpage,
                      tabs: Vec<(Tabpage, Option<&str>)>)
                      -> RepaintMode;

    fn mode_info_set(&mut self,
                     cursor_style_enabled: bool,
                     mode_info: Vec<ModeInfo>)
                     -> RepaintMode;
}

pub trait GuiApi {
    fn set_font(&mut self, font_desc: &str);
}

macro_rules! try_str {
    ($exp:expr) => ($exp.as_str().ok_or("Can't convert argument to string".to_owned())?)
}

macro_rules! try_int {
    ($expr:expr) => ($expr.as_i64().ok_or("Can't convert argument to int".to_owned())?)
}

macro_rules! try_uint {
    ($exp:expr) => ($exp.as_u64().ok_or("Can't convert argument to u64".to_owned())?)
}

macro_rules! try_bool {
    ($exp:expr) => ($exp.as_bool().ok_or("Can't convert argument to bool".to_owned())?)
}

pub enum CursorShape {
    Block,
    Horizontal,
    Vertical,
    Unknown,
}

impl CursorShape {
    fn new(shape_code: &Value) -> Result<CursorShape, String> {
        let str_code = shape_code
            .as_str()
            .ok_or("Can't convert cursor shape to string".to_owned())?;

        Ok(match str_code {
               "block" => CursorShape::Block,
               "horizontal" => CursorShape::Horizontal,
               "vertical" => CursorShape::Vertical,
               _ => {
                   error!("Unknown cursor_shape {}", str_code);
                   CursorShape::Unknown
               }
           })
    }
}

pub struct ModeInfo {
    cursor_shape: Option<CursorShape>,
}

impl ModeInfo {
    pub fn new(mode_info_arr: &Vec<(Value, Value)>) -> Result<Self, String> {
        let mode_info_map = mode_info_arr
            .iter()
            .map(|p| {
                     p.0
                         .as_str()
                         .ok_or("mode_info key not string".to_owned())
                         .map(|key| (key, p.1.clone()))
                 })
            .collect::<Result<HashMap<&str, Value>, String>>()?;

        let cursor_shape = if let Some(shape) = mode_info_map.get("cursor_shape") {
            Some(CursorShape::new(shape)?)
        } else {
            None
        };

        Ok(ModeInfo { cursor_shape })
    }
}

#[derive(Debug)]
pub struct NvimInitError {
    source: Box<error::Error>,
    cmd: String,
}

impl NvimInitError {
    pub fn new<E>(cmd: &Command, error: E) -> NvimInitError
        where E: Into<Box<error::Error>>
    {
        NvimInitError {
            cmd: format!("{:?}", cmd),
            source: error.into(),
        }
    }

    pub fn source(&self) -> String {
        format!("{}", self.source)
    }

    pub fn cmd(&self) -> &str {
        &self.cmd
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

pub fn initialize(shell: Arc<UiMutex<shell::State>>,
                  nvim_bin_path: Option<&String>,
                  cols: u64,
                  rows: u64)
                  -> result::Result<Neovim, NvimInitError> {
    let mut cmd = if let Some(path) = nvim_bin_path {
        Command::new(path)
    } else {
        Command::new("nvim")
    };

    cmd.arg("--embed")
        .arg("--headless")
        .arg("--cmd")
        .arg("set termguicolors")
        .arg("--cmd")
        .arg("let g:GtkGuiLoaded = 1")
        .stderr(Stdio::inherit());

    if let Ok(runtime_path) = env::var("NVIM_GTK_RUNTIME_PATH") {
        cmd.arg("--cmd")
            .arg(format!("let &rtp.=',{}'", runtime_path));
    } else if let Some(prefix) = option_env!("PREFIX") {
        cmd.arg("--cmd")
            .arg(format!("let &rtp.=',{}/share/nvim-gtk/runtime'", prefix));
    } else {
        cmd.arg("--cmd").arg("let &rtp.=',runtime'");
    }

    let session = Session::new_child_cmd(&mut cmd);

    let session = match session {
        Err(e) => return Err(NvimInitError::new(&cmd, e)),
        Ok(s) => s,
    };

    let mut nvim = Neovim::new(session);

    nvim.session
        .start_event_loop_handler(NvimHandler::new(shell));
    let mut opts = UiAttachOptions::new();
    opts.set_popupmenu_external(false);
    opts.set_tabline_external(true);
    nvim.ui_attach(cols, rows, opts)
        .map_err(|e| NvimInitError::new(&cmd, e))?;
    nvim.command("runtime! ginit.vim")
        .map_err(|e| NvimInitError::new(&cmd, e))?;

    Ok(nvim)
}

pub struct NvimHandler {
    shell: Arc<UiMutex<shell::State>>,
}

impl NvimHandler {
    pub fn new(shell: Arc<UiMutex<shell::State>>) -> NvimHandler {
        NvimHandler { shell: shell }
    }

    fn nvim_cb(&self, method: &str, params: Vec<Value>) {
        match method {
            "redraw" => {
                self.safe_call(move |ui| {
                    let mut repaint_mode = RepaintMode::Nothing;

                    for ev in &params {
                        if let Some(ev_args) = ev.as_array() {
                            if let Some(ev_name) = ev_args[0].as_str() {
                                for ref local_args in ev_args.iter().skip(1) {
                                    let args = match *local_args {
                                        &Value::Array(ref ar) => ar.clone(),
                                        _ => vec![],
                                    };
                                    let call_reapint_mode = call(ui, ev_name, &args)?;
                                    repaint_mode = repaint_mode.join(call_reapint_mode);
                                }
                            } else {
                                println!("Unsupported event {:?}", ev_args);
                            }
                        } else {
                            println!("Unsupported event type {:?}", ev);
                        }
                    }

                    ui.on_redraw(&repaint_mode);
                    Ok(())
                });
            }
            "Gui" => {
                if params.len() > 0 {
                    if let Some(ev_name) = params[0].as_str().map(String::from) {
                        let args = params.iter().skip(1).cloned().collect();
                        self.safe_call(move |ui| {
                                           call_gui_event(ui, &ev_name, &args)?;
                                           ui.on_redraw(&RepaintMode::All);
                                           Ok(())
                                       });
                    } else {
                        println!("Unsupported event {:?}", params);
                    }
                } else {
                    println!("Unsupported event {:?}", params);
                }
            }
            _ => {
                println!("Notification {}({:?})", method, params);
            }
        }
    }

    fn safe_call<F>(&self, cb: F)
        where F: Fn(&mut shell::State) -> result::Result<(), String> + 'static + Send
    {
        let shell = self.shell.clone();
        glib::idle_add(move || {
                           if let Err(msg) = cb(&mut shell.borrow_mut()) {
                               println!("Error call function: {}", msg);
                           }
                           glib::Continue(false)
                       });
    }
}

impl Handler for NvimHandler {
    fn handle_notify(&mut self, name: &str, args: &Vec<Value>) {
        self.nvim_cb(name, args.clone());
    }
}


fn call_gui_event(ui: &mut shell::State,
                  method: &str,
                  args: &Vec<Value>)
                  -> result::Result<(), String> {
    match method {
        "Font" => ui.set_font(try_str!(args[0])),
        "Option" => {
            match try_str!(args[0]) {
                "Popupmenu" => {
                    ui.nvim()
                        .set_option(UiOption::ExtPopupmenu(try_uint!(args[1]) == 1))
                        .map_err(|e| e.to_string())?
                }
                "Tabline" => {
                    ui.nvim()
                        .set_option(UiOption::ExtTabline(try_uint!(args[1]) == 1))
                        .map_err(|e| e.to_string())?
                }
                opt => error!("Unknown option {}", opt),
            }
        }
        _ => return Err(format!("Unsupported event {}({:?})", method, args)),
    }
    Ok(())
}

fn call(ui: &mut shell::State,
        method: &str,
        args: &Vec<Value>)
        -> result::Result<RepaintMode, String> {
    let repaint_mode = match method {
        "cursor_goto" => ui.on_cursor_goto(try_uint!(args[0]), try_uint!(args[1])),
        "put" => ui.on_put(try_str!(args[0])),
        "clear" => ui.on_clear(),
        "resize" => ui.on_resize(try_uint!(args[0]), try_uint!(args[1])),
        "highlight_set" => {
            if let Value::Map(ref attrs) = args[0] {
                ui.on_highlight_set(attrs);
            } else {
                panic!("Supports only map value as argument");
            }
            RepaintMode::Nothing
        }
        "eol_clear" => ui.on_eol_clear(),
        "set_scroll_region" => {
            ui.on_set_scroll_region(try_uint!(args[0]),
                                    try_uint!(args[1]),
                                    try_uint!(args[2]),
                                    try_uint!(args[3]));
            RepaintMode::Nothing
        }
        "scroll" => ui.on_scroll(try_int!(args[0])),
        "update_bg" => ui.on_update_bg(try_int!(args[0])),
        "update_fg" => ui.on_update_fg(try_int!(args[0])),
        "update_sp" => ui.on_update_sp(try_int!(args[0])),
        "mode_change" => ui.on_mode_change(try_str!(args[0])),
        "mouse_on" => ui.on_mouse(true),
        "mouse_off" => ui.on_mouse(false),
        "busy_start" => ui.on_busy(true),
        "busy_stop" => ui.on_busy(false),
        "popupmenu_show" => {
            let mut menu_items = Vec::new();

            let items = args[0].as_array().ok_or("Error get menu list array")?;
            for item in items {
                let item_line: result::Result<Vec<_>, &str> = item.as_array()
                    .ok_or("Error get menu item array")?
                    .iter()
                    .map(|col| col.as_str().ok_or("Error get menu column"))
                    .collect();
                menu_items.push(item_line?);
            }

            ui.popupmenu_show(&menu_items,
                              try_int!(args[1]),
                              try_uint!(args[2]),
                              try_uint!(args[3]))
        }
        "popupmenu_hide" => ui.popupmenu_hide(),
        "popupmenu_select" => ui.popupmenu_select(try_int!(args[0])),
        "tabline_update" => {
            let tabs_in = args[1].as_array().ok_or("Error get tabline list")?;

            let mut tabs_out = Vec::new();
            for tab in tabs_in {
                let tab_attrs = tab.as_map().ok_or("Error get map for tab")?;

                let mut tab_attr = None;
                let mut name_attr = None;

                for attr in tab_attrs {
                    let key = attr.0.as_str().ok_or("Error get key value")?;
                    if key == "tab" {
                        tab_attr = Some(Tabpage::new(attr.1.clone()));
                    } else if key == "name" {
                        name_attr = attr.1.as_str();
                    }
                }
                tabs_out.push((tab_attr.unwrap(), name_attr));
            }
            ui.tabline_update(Tabpage::new(args[0].clone()), tabs_out)
        }
        "mode_info_set" => {
            let mode_info_array = args[1]
                .as_array()
                .ok_or("Erro get array key value for mode_info")?;

            let mode_info = mode_info_array
                .iter()
                .map(|mi| {
                         mi.as_map()
                             .ok_or("Erro get map for mode_info".to_owned())
                             .and_then(|mi_map| ModeInfo::new(mi_map))
                     })
                .collect::<Result<Vec<_>, String>>()?;
            ui.mode_info_set(try_bool!(args[0]), mode_info)
        }
        _ => {
            println!("Event {}({:?})", method, args);
            RepaintMode::Nothing
        }
    };

    Ok(repaint_mode)
}

pub trait ErrorReport {
    fn report_err(&self, nvim: &mut NeovimApi);
}

impl<T> ErrorReport for result::Result<T, CallError> {
    fn report_err(&self, _: &mut NeovimApi) {
        if let &Err(ref err) = self {
            println!("{}", err);
            //nvim.report_error(&err_msg).expect("Error report error :)");
        }
    }
}

#[derive(Clone, Debug)]
pub enum RepaintMode {
    Nothing,
    All,
    AreaList(ModelRectVec),
    Area(ModelRect),
}

impl RepaintMode {
    pub fn join(self, mode: RepaintMode) -> RepaintMode {
        match (self, mode) {
            (RepaintMode::Nothing, m) => m,
            (m, RepaintMode::Nothing) => m,
            (RepaintMode::All, _) => RepaintMode::All,
            (_, RepaintMode::All) => RepaintMode::All,
            (RepaintMode::Area(mr1), RepaintMode::Area(mr2)) => {
                let mut vec = ModelRectVec::new(mr1);
                vec.join(&mr2);
                RepaintMode::AreaList(vec)
            }
            (RepaintMode::AreaList(mut target), RepaintMode::AreaList(source)) => {
                for s in &source.list {
                    target.join(&s);
                }
                RepaintMode::AreaList(target)
            }
            (RepaintMode::AreaList(mut list), RepaintMode::Area(l2)) => {
                list.join(&l2);
                RepaintMode::AreaList(list)
            }
            (RepaintMode::Area(l1), RepaintMode::AreaList(mut list)) => {
                list.join(&l1);
                RepaintMode::AreaList(list)
            }
        }
    }
}


enum NeovimClientWrapper {
    Uninitialized,
    Initialized(Neovim),
    Error,
}

impl NeovimClientWrapper {
    pub fn is_initialized(&self) -> bool {
        match *self {
            NeovimClientWrapper::Initialized(_) => true,
            _ => false,
        }
    }

    pub fn is_error(&self) -> bool {
        match *self {
            NeovimClientWrapper::Error => true,
            _ => false,
        }
    }

    pub fn nvim(&self) -> &Neovim {
        match *self {
            NeovimClientWrapper::Initialized(ref nvim) => nvim,
            NeovimClientWrapper::Uninitialized => panic!("Access to uninitialized neovim client"),
            NeovimClientWrapper::Error => {
                panic!("Access to neovim client that is not started due to some error")
            }
        }
    }

    pub fn nvim_mut(&mut self) -> &mut Neovim {
        match *self {
            NeovimClientWrapper::Initialized(ref mut nvim) => nvim,
            NeovimClientWrapper::Uninitialized => panic!("Access to uninitialized neovim client"),
            NeovimClientWrapper::Error => {
                panic!("Access to neovim client that is not started due to some error")
            }
        }
    }
}

pub struct NeovimClient {
    nvim: NeovimClientWrapper,
}

impl NeovimClient {
    pub fn new() -> Self {
        NeovimClient { nvim: NeovimClientWrapper::Uninitialized }
    }

    pub fn set_nvim(&mut self, nvim: Neovim) {
        self.nvim = NeovimClientWrapper::Initialized(nvim);
    }

    pub fn set_error(&mut self) {
        self.nvim = NeovimClientWrapper::Error;
    }

    pub fn is_initialized(&self) -> bool {
        self.nvim.is_initialized()
    }

    pub fn is_error(&self) -> bool {
        self.nvim.is_error()
    }

    pub fn nvim(&self) -> &Neovim {
        self.nvim.nvim()
    }

    pub fn nvim_mut(&mut self) -> &mut Neovim {
        self.nvim.nvim_mut()
    }
}

impl Deref for NeovimClient {
    type Target = Neovim;

    fn deref(&self) -> &Neovim {
        self.nvim()
    }
}

impl DerefMut for NeovimClient {
    fn deref_mut(&mut self) -> &mut Neovim {
        self.nvim_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode() {
        let mode = RepaintMode::Area(ModelRect::point(1, 1));
        let mode = mode.join(RepaintMode::Nothing);

        match mode {
            RepaintMode::Area(ref rect) => {
                assert_eq!(1, rect.top);
                assert_eq!(1, rect.bot);
                assert_eq!(1, rect.left);
                assert_eq!(1, rect.right);
            }
            _ => panic!("mode is worng"),
        }
    }
}
