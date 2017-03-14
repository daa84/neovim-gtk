use neovim_lib::{Neovim, NeovimApi, Session, Value, Integer, UiAttachOptions, CallError};
use std::io::{Result, Error, ErrorKind};
use std::result;
use ui_model::UiModel;
use ui;
use ui::Ui;
use glib;

pub trait RedrawEvents {
    fn on_cursor_goto(&mut self, row: u64, col: u64);

    fn on_put(&mut self, text: &str);

    fn on_clear(&mut self);

    fn on_resize(&mut self, columns: u64, rows: u64);

    fn on_redraw(&self);

    fn on_highlight_set(&mut self, attrs: &Vec<(Value, Value)>);

    fn on_eol_clear(&mut self);

    fn on_set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64);

    fn on_scroll(&mut self, count: i64);

    fn on_update_bg(&mut self, bg: i64);

    fn on_update_fg(&mut self, fg: i64);

    fn on_update_sp(&mut self, sp: i64);

    fn on_mode_change(&mut self, mode: &str);

    fn on_mouse_on(&mut self);

    fn on_mouse_off(&mut self);
}

pub trait GuiApi {
    fn set_font(&mut self, font_desc: &str);
}

macro_rules! try_str {
    ($exp:expr) => (match $exp {
        Value::String(ref val) => val,
        _ => return Err("Can't convert argument to string".to_owned())
    })
}

macro_rules! try_int {
    ($expr:expr) => (match $expr {
        Value::Integer(Integer::U64(val)) => val as i64,
        Value::Integer(Integer::I64(val)) => val,
        _ =>  return Err("Can't convert argument to int".to_owned())
    })
}

macro_rules! try_uint {
    ($exp:expr) => (match $exp {
        Value::Integer(Integer::U64(val)) => val,
        _ => return Err("Can't convert argument to u64".to_owned())
    })
}

pub fn initialize(ui: &mut Ui, nvim_bin_path: Option<&String>, open_arg: Option<&String>) -> Result<()> {
    let session = if let Some(path) = nvim_bin_path {
        Session::new_child_path(path)?
    } else {
        Session::new_child()?
    };

    let nvim = Neovim::new(session);
    ui.set_nvim(nvim);
    ui.model = UiModel::new(24, 80);

    let mut nvim = ui.nvim();

    nvim.session.start_event_loop_cb(move |m, p| nvim_cb(m, p));
    nvim.ui_attach(80, 24, UiAttachOptions::new()).map_err(|e| Error::new(ErrorKind::Other, e))?;
    nvim.command("runtime! ginit.vim").map_err(|e| Error::new(ErrorKind::Other, e))?;

    if let Some(ref file) = open_arg {
        nvim.command(&format!("e {}", file)).report_err(nvim);
    }

    Ok(())
}

fn nvim_cb(method: &str, params: Vec<Value>) {
    match method {
        "redraw" => {
            safe_call(move |ui| {
                for ev in &params {
                    if let &Value::Array(ref ev_args) = ev {
                        if let Value::String(ref ev_name) = ev_args[0] {
                            for ref local_args in ev_args.iter().skip(1) {
                                let args = match *local_args {
                                    &Value::Array(ref ar) => ar.clone(),
                                    _ => vec![],
                                };
                                call(ui, ev_name, &args)?;
                            }
                        } else {
                            println!("Unsupported event {:?}", ev_args);
                        }
                    } else {
                        println!("Unsupported event type {:?}", ev);
                    }
                }

                ui.on_redraw();
                Ok(())
            });
        }
        "Gui" => {
            if params.len() > 0 {
                if let Value::String(ev_name) = params[0].clone() {
                    let args = params.iter().skip(1).cloned().collect();
                    safe_call(move |ui| {
                        call_gui_event(ui, &ev_name, &args)?;
                        ui.on_redraw();
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

fn call_gui_event(ui: &mut Ui, method: &str, args: &Vec<Value>) -> result::Result<(), String> {
    match method {
        "Font" => ui.set_font(try_str!(args[0])),
        _ => return Err(format!("Unsupported event {}({:?})", method, args)),
    }
    Ok(())
}

fn call(ui: &mut Ui, method: &str, args: &Vec<Value>) -> result::Result<(), String> {
    match method {
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
        }
        "eol_clear" => ui.on_eol_clear(),
        "set_scroll_region" => {
            ui.on_set_scroll_region(try_uint!(args[0]),
                                    try_uint!(args[1]),
                                    try_uint!(args[2]),
                                    try_uint!(args[3]));
        }
        "scroll" => ui.on_scroll(try_int!(args[0])),
        "update_bg" => ui.on_update_bg(try_int!(args[0])),
        "update_fg" => ui.on_update_fg(try_int!(args[0])),
        "update_sp" => ui.on_update_sp(try_int!(args[0])),
        "mode_change" => ui.on_mode_change(try_str!(args[0])),
        "mouse_on" => ui.on_mouse_on(),
        "mouse_off" => ui.on_mouse_off(),
        _ => println!("Event {}({:?})", method, args),
    };

    Ok(())
}

fn safe_call<F>(cb: F)
    where F: Fn(&mut Ui) -> result::Result<(), String> + 'static + Send
{
    glib::idle_add(move || {
        ui::UI.with(|ui_cell| if let Err(msg) = cb(&mut *ui_cell.borrow_mut()) {
            println!("Error call function: {}", msg);
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
