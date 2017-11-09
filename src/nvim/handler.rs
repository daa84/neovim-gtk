use std::result;
use std::sync::Arc;

use neovim_lib::{Handler, Value};

use ui::UiMutex;
use shell;
use glib;

use super::repaint_mode::RepaintMode;
use super::redraw_handler;
use super::redraw_handler::RedrawEvents;

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

                    for ev in params {
                        if let Some(ev_args) = ev.as_array() {
                            if let Some(ev_name) = ev_args[0].as_str() {
                                for local_args in ev_args.iter().skip(1) {
                                    let args = match *local_args {
                                        Value::Array(ref ar) => ar.clone(),
                                        _ => vec![],
                                    };
                                    let call_reapint_mode =
                                        redraw_handler::call(ui, ev_name, &args)?;
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
                if !params.is_empty() {
                    if let Some(ev_name) = params[0].as_str().map(String::from) {
                        let args = params.iter().skip(1).cloned().collect();
                        self.safe_call(move |ui| {
                            redraw_handler::call_gui_event(ui, &ev_name, &args)?;
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
    where
        F: FnOnce(&mut shell::State) -> result::Result<(), String> + 'static + Send,
    {
        let mut cb = Some(cb);
        let shell = self.shell.clone();
        glib::idle_add(move || {
            if let Err(msg) = cb.take().unwrap()(&mut shell.borrow_mut()) {
                println!("Error call function: {}", msg);
            }
            glib::Continue(false)
        });
    }
}

impl Handler for NvimHandler {
    fn handle_notify(&mut self, name: &str, args: Vec<Value>) {
        self.nvim_cb(name, args);
    }
}
