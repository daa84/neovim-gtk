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
                        if let Value::Array(ev_args) = ev {
                            let mut args_iter = ev_args.into_iter();
                            let ev_name = args_iter.next();
                            if let Some(ev_name) = ev_name {
                                if let Some(ev_name) = ev_name.as_str() {
                                    for local_args in args_iter {
                                        let args = match local_args {
                                            Value::Array(ar) => ar,
                                            _ => vec![],
                                        };
                                        let call_reapint_mode =
                                            redraw_handler::call(ui, &ev_name, &args)?;
                                        repaint_mode = repaint_mode.join(call_reapint_mode);
                                    }
                                } else {
                                    error!("Unsupported event");
                                }
                            } else {
                                error!("Event name does not exists");
                            }
                        } else {
                            error!("Unsupported event type {:?}", ev);
                        }
                    }

                    ui.on_redraw(&repaint_mode);
                    Ok(())
                });
            }
            "Gui" => {
                if !params.is_empty() {
                    let mut params_iter = params.into_iter();
                    if let Some(ev_name) = params_iter.next() {
                        if let Value::String(ev_name) = ev_name {
                            let args = params_iter.collect();
                            self.safe_call(move |ui| {
                                redraw_handler::call_gui_event(
                                    ui,
                                    ev_name.as_str().ok_or_else(|| "Event name does not exists")?,
                                    &args,
                                )?;
                                ui.on_redraw(&RepaintMode::All);
                                Ok(())
                            });
                        } else {
                            error!("Unsupported event");
                        }
                    } else {
                        error!("Event name does not exists");
                    }
                } else {
                    error!("Unsupported event {:?}", params);
                }
            }
            _ => {
                error!("Notification {}({:?})", method, params);
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
                error!("Error call function: {}", msg);
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
