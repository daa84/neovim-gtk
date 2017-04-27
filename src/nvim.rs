use std::io::{Result, Error, ErrorKind};
use std::result;
use std::sync::Arc;

use ui::UiMutex;
use neovim_lib::{Handler, Neovim, NeovimApi, Session, Value, UiAttachOptions, CallError};
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

pub fn initialize(shell: Arc<UiMutex<shell::State>>,
                  nvim_bin_path: Option<&String>,
                  external_popup: bool)
                  -> Result<Neovim> {
    let session = if let Some(path) = nvim_bin_path {
        match Session::new_child_path(path) {
            Err(e) => {
                println!("Error execute {}", path);
                return Err(From::from(e));
            }
            Ok(s) => s,
        }
    } else {
        Session::new_child()?
    };

    let mut nvim = Neovim::new(session);

    nvim.session
        .start_event_loop_handler(NvimHandler::new(shell));
    let mut opts = UiAttachOptions::new();
    opts.set_popupmenu_external(external_popup);
    nvim.ui_attach(80, 24, opts)
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    nvim.command("runtime! ginit.vim")
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

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
