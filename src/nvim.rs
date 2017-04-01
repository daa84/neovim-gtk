use neovim_lib::{Handler, Neovim, NeovimApi, Session, Value, UiAttachOptions, CallError};
use std::io::{Result, Error, ErrorKind};
use std::result;
use ui_model::{UiModel, ModelRect};
use ui::SH;
use shell::Shell;
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

    fn on_mouse_on(&mut self) -> RepaintMode;

    fn on_mouse_off(&mut self) -> RepaintMode;
}

pub trait GuiApi {
    fn set_font(&mut self, font_desc: &str);
}

macro_rules! try_str {
    ($exp:expr) => (match $exp.as_str() {
        Some(val) => val,
        _ => return Err("Can't convert argument to string".to_owned())
    })
}

macro_rules! try_int {
    ($expr:expr) => (match $expr.as_i64() {
        Some(val) => val,
        _ =>  return Err("Can't convert argument to int".to_owned())
    })
}

macro_rules! try_uint {
    ($exp:expr) => (match $exp.as_u64() {
        Some(val) => val,
        _ => return Err("Can't convert argument to u64".to_owned())
    })
}

pub fn initialize(ui: &mut Shell, nvim_bin_path: Option<&String>) -> Result<()> {
    let session = if let Some(path) = nvim_bin_path {
        Session::new_child_path(path)?
    } else {
        Session::new_child()?
    };

    let nvim = Neovim::new(session);
    ui.set_nvim(nvim);
    ui.model = UiModel::new(24, 80);

    let mut nvim = ui.nvim();

    nvim.session.start_event_loop_handler(NvimHandler::new());
    nvim.ui_attach(80, 24, UiAttachOptions::new()).map_err(|e| Error::new(ErrorKind::Other, e))?;
    nvim.command("runtime! ginit.vim").map_err(|e| Error::new(ErrorKind::Other, e))?;

    Ok(())
}

pub fn open_file(nvim: &mut NeovimApi, file: Option<&String>) {
    if let Some(file_name) = file {
        nvim.command(&format!("e {}", file_name)).report_err(nvim);
    }
}

pub struct NvimHandler {}

impl NvimHandler {
    pub fn new() -> NvimHandler {
        NvimHandler {}
    }
}

impl Handler for NvimHandler {
    fn handle_notify(&mut self, name: &str, args: &Vec<Value>) {
        nvim_cb(name, args.clone());
    }
}

fn nvim_cb(method: &str, params: Vec<Value>) {
    match method {
        "redraw" => {
            safe_call(move |ui| {
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
                                repaint_mode = repaint_mode.join(&call_reapint_mode);
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
                    safe_call(move |ui| {
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

fn call_gui_event(ui: &mut Shell, method: &str, args: &Vec<Value>) -> result::Result<(), String> {
    match method {
        "Font" => ui.set_font(try_str!(args[0])),
        _ => return Err(format!("Unsupported event {}({:?})", method, args)),
    }
    Ok(())
}

fn call(ui: &mut Shell, method: &str, args: &Vec<Value>) -> result::Result<RepaintMode, String> {
    Ok(match method {
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
        "mouse_on" => ui.on_mouse_on(),
        "mouse_off" => ui.on_mouse_off(),
        _ => {
            println!("Event {}({:?})", method, args);
            RepaintMode::Nothing
        }
    })
}

fn safe_call<F>(cb: F)
    where F: Fn(&mut Shell) -> result::Result<(), String> + 'static + Send
{
    glib::idle_add(move || {
        SHELL!(shell = {
            if let Err(msg) = cb(&mut shell) {
                println!("Error call function: {}", msg);
            }
        });
        glib::Continue(false)
    });
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

#[derive(Clone)]
pub enum RepaintMode {
    Nothing,
    All,
    Area(ModelRect),
}

impl RepaintMode {
    pub fn join(&self, mode: &RepaintMode) -> RepaintMode {
        match (self, mode) {
            (&RepaintMode::Nothing, m) => m.clone(),
            (m, &RepaintMode::Nothing) => m.clone(),
            (&RepaintMode::All, _) => RepaintMode::All,
            (_, &RepaintMode::All) => RepaintMode::All,
            (&RepaintMode::Area(ref mr1), &RepaintMode::Area(ref mr2)) => {
                let mut area = mr1.clone();
                area.join(mr2);
                RepaintMode::Area(area)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode() {
        let mode = RepaintMode::Area(ModelRect::point(1, 1));
        mode.join(&RepaintMode::Nothing);

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
