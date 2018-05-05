use std::collections::HashMap;
use std::sync::Arc;
use std::cell::Ref;

use glib;

use neovim_lib::{CallError, Neovim, NeovimApiAsync, Value};

use ui::UiMutex;
use nvim::ErrorReport;
use color::Color;
use value::ValueMapExt;

struct State {
    pmenu: Pmenu,
    cursor: Cursor,
}

impl State {
    fn new() -> Self {
        State {
            pmenu: Pmenu::new(),
            cursor: Cursor::new(),
        }
    }
}

pub struct Theme {
    state: Arc<UiMutex<State>>,
}

impl Theme {
    pub fn new() -> Self {
        Theme {
            state: Arc::new(UiMutex::new(State::new())),
        }
    }

    pub fn pmenu(&self) -> Ref<Pmenu> {
        Ref::map(self.state.borrow(), |s| &s.pmenu)
    }

    pub fn cursor(&self) -> Ref<Cursor> {
        Ref::map(self.state.borrow(), |s| &s.cursor)
    }

    pub fn queue_update(&self, nvim: &mut Neovim) {
        self.get_hl(nvim, "Cursor", |state, bg, _fg| {
            state.cursor.bg = bg;
        });

        self.get_hl(nvim, "Pmenu", |state, bg, fg| {
            state.pmenu.bg = bg;
            state.pmenu.fg = fg;
        });

        self.get_hl(nvim, "PmenuSel", |state, bg_sel, fg_sel| {
            state.pmenu.bg_sel = bg_sel;
            state.pmenu.fg_sel = fg_sel;
        });
    }

    fn get_hl<CB>(&self, nvim: &mut Neovim, hl_name: &str, mut cb: CB)
    where
        CB: FnMut(&mut State, Option<Color>, Option<Color>) + Send + 'static,
    {
        let state = self.state.clone();

        nvim.get_hl_by_name_async(hl_name, true)
            .cb(move |v| {
                let mut hl = Some(hl_colors(v));
                glib::idle_add(move || {
                    let (bg, fg) = hl.take().unwrap();
                    let mut state = state.borrow_mut();
                    cb(&mut *state, bg, fg);
                    glib::Continue(false)
                });
            })
            .call();
    }
}

pub struct Cursor {
    pub bg: Option<Color>,
}

impl Cursor {
    pub fn new() -> Self {
        Cursor {  
            bg: None,
        }
    }
}

pub struct Pmenu {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub bg_sel: Option<Color>,
    pub fg_sel: Option<Color>,
}

impl Pmenu {
    pub fn new() -> Self {
        Pmenu {
            bg: None,
            fg: None,
            bg_sel: None,
            fg_sel: None,
        }
    }
}

fn get_hl_color(map: &HashMap<&str, &Value>, color_name: &str) -> Option<Color> {
    map.get(color_name)
        .and_then(|col| col.as_u64())
        .map(Color::from_indexed_color)
}

fn hl_colors(hl: Result<Vec<(Value, Value)>, CallError>) -> (Option<Color>, Option<Color>) {
    hl.ok_and_report()
        .and_then(|m| {
            if let Some(m) = m.to_attrs_map_report() {
                let reverse = m.get("reverse").and_then(|v| v.as_bool()).unwrap_or(false);
                let bg = get_hl_color(&m, "background");
                let fg = get_hl_color(&m, "foreground");
                if reverse {
                    Some((fg, bg))
                } else {
                    Some((bg, fg))
                }
            } else {
                None
            }
        })
        .unwrap_or((None, None))
}

