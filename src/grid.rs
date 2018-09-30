use std::ops::{Deref, Index, IndexMut};
use std::rc::Rc;

use gdk;
use gtk::{self, prelude::*};

use fnv::FnvHashMap;

use neovim_lib::Value;

use highlight::{Highlight, HighlightMap};
use nvim::{RepaintGridEvent, RepaintMode};
use render;
use shell::RenderState;
use ui_model::{ModelRect, ModelRectVec, UiModel};

const DEFAULT_GRID: u64 = 1;

type ButtonEventCb = Fn(u64, &gdk::EventButton) + 'static;
type KeyEventCb = Fn(u64, &gdk::EventKey) -> Inhibit + 'static;
type ScrollEventCb = Fn(u64, &gdk::EventScroll) + 'static;

struct Callbacks {
    button_press_cb: Option<Box<ButtonEventCb>>,
    button_release_cb: Option<Box<ButtonEventCb>>,
    key_press_cb: Option<Box<KeyEventCb>>,
    key_release_cb: Option<Box<KeyEventCb>>,
    scroll_cb: Option<Box<ScrollEventCb>>,
}

impl Callbacks {
    pub fn new() -> Self {
        Callbacks {
            button_press_cb: None,
            button_release_cb: None,
            key_press_cb: None,
            key_release_cb: None,
            scroll_cb: None,
        }
    }
}

pub struct GridMap {
    grids: FnvHashMap<u64, Grid>,
    fixed: gtk::Fixed,

    callbacks: Rc<Callbacks>,
}

impl Index<u64> for GridMap {
    type Output = Grid;

    fn index(&self, idx: u64) -> &Grid {
        &self.grids[&idx]
    }
}

impl IndexMut<u64> for GridMap {
    fn index_mut(&mut self, idx: u64) -> &mut Grid {
        self.grids.get_mut(&idx).unwrap()
    }
}

impl GridMap {
    pub fn new() -> Self {
        let fixed = gtk::Fixed::new();
        fixed.set_hexpand(true);
        fixed.set_vexpand(true);

        GridMap {
            grids: FnvHashMap::default(),
            fixed,

            callbacks: Rc::new(Callbacks::new()),
        }
    }

    pub fn queue_redraw_all(&mut self, render_state: &RenderState) {
        for grid_id in self.grids.keys() {
            self.queue_redraw(
                render_state,
                &RepaintGridEvent::new(*grid_id, RepaintMode::All),
            );
        }
    }

    pub fn queue_redraw(&mut self, render_state: &RenderState, ev: &RepaintGridEvent) {
        if let Some(grid) = self.grids.get(&ev.grid_id.unwrap()) {
            match ev.mode {
                RepaintMode::All => {
                    grid.update_dirty_glyphs(render_state);
                    grid.drawing_area.queue_draw();
                }
                RepaintMode::Area(ref rect) => grid.queue_draw_area(render_state, &[rect]),
                RepaintMode::AreaList(ref list) => grid.queue_draw_area(render_state, &list.list),
                RepaintMode::Nothing => (),
            }
        } else {
            warn!("Event from no known grid {:?}", ev.grid_id);
        }
    }

    pub fn current(&self) -> Option<&Grid> {
        self.grids.get(&DEFAULT_GRID)
    }

    pub fn current_model_mut(&mut self) -> Option<&mut UiModel> {
        self.grids.get_mut(&DEFAULT_GRID).map(|g| &mut g.model)
    }

    pub fn current_model(&self) -> Option<&UiModel> {
        self.grids.get(&DEFAULT_GRID).map(|g| &g.model)
    }

    pub fn get_or_create(&mut self, idx: u64) -> &mut Grid {
        if self.grids.contains_key(&idx) {
            return self.grids.get_mut(&idx).unwrap();
        }

        let grid = Grid::new(idx);
        self.fixed.put(&*grid, 0, 0);

        let cbs = self.callbacks.clone();
        grid.connect_button_press_event(move |_, ev| {
            cbs.button_press_cb.map(|cb| cb(idx, ev));
            Inhibit(false)
        });

        let cbs = self.callbacks.clone();
        grid.connect_button_release_event(move |_, ev| {
            cbs.button_release_cb.map(|cb| cb(idx, ev));
            Inhibit(false)
        });

        let cbs = self.callbacks.clone();
        grid.connect_key_press_event(move |_, ev| {
            cbs.key_press_cb
                .map(|cb| cb(idx, ev))
                .unwrap_or(Inhibit(false))
        });

        let cbs = self.callbacks.clone();
        grid.connect_key_release_event(move |_, ev| {
            cbs.key_release_cb
                .map(|cb| cb(idx, ev))
                .unwrap_or(Inhibit(false))
        });

        let cbs = self.callbacks.clone();
        grid.connect_scroll_event(move |_, ev| {
            cbs.scroll_cb.map(|cb| cb(idx, ev));
            Inhibit(false)
        });

        self.grids.insert(idx, grid);
        self.grids.get_mut(&idx).unwrap()
    }

    pub fn destroy(&mut self, idx: u64) {
        self.grids.remove(&idx);
    }

    pub fn clear_glyphs(&mut self) {
        for grid in self.grids.values_mut() {
            grid.model.clear_glyphs();
        }
    }
}

impl GridMap {
    pub fn connect_button_press_event<T>(&mut self, cb: T)
    where
        T: Fn(u64, &gdk::EventButton) + 'static,
    {
        Rc::get_mut(&mut self.callbacks).unwrap().button_press_cb = Some(Box::new(cb));
    }

    pub fn connect_button_release_event<T>(&mut self, cb: T)
    where
        T: Fn(u64, &gdk::EventButton) + 'static,
    {
        Rc::get_mut(&mut self.callbacks).unwrap().button_release_cb = Some(Box::new(cb));
    }

    pub fn connect_key_press_event<T>(&mut self, cb: T)
    where
        T: Fn(u64, &gdk::EventKey) -> Inhibit + 'static,
    {
        Rc::get_mut(&mut self.callbacks).unwrap().key_press_cb = Some(Box::new(cb));
    }

    pub fn connect_key_release_event<T>(&mut self, cb: T)
    where
        T: Fn(u64, &gdk::EventKey) -> Inhibit + 'static,
    {
        Rc::get_mut(&mut self.callbacks).unwrap().key_release_cb = Some(Box::new(cb));
    }

    pub fn connect_scroll_event<T>(&mut self, cb: T)
    where
        T: Fn(u64, &gdk::EventScroll) + 'static,
    {
        Rc::get_mut(&mut self.callbacks).unwrap().scroll_cb = Some(Box::new(cb));
    }
}

impl Deref for GridMap {
    type Target = gtk::Fixed;

    fn deref(&self) -> &gtk::Fixed {
        &self.fixed
    }
}

pub struct Grid {
    grid: u64,
    model: UiModel,
    drawing_area: gtk::DrawingArea,
}

impl Grid {
    pub fn queue_draw_area<M: AsRef<ModelRect>>(
        &mut self,
        render_state: &RenderState,
        rect_list: &[M],
    ) {
        // extends by items before, then after changes

        let rects: Vec<_> = rect_list
            .iter()
            .map(|rect| rect.as_ref().clone())
            .map(|mut rect| {
                rect.extend_by_items(&self.model);
                rect
            }).collect();

        self.update_dirty_glyphs(&render_state);

        let cell_metrics = render_state.font_ctx.cell_metrics();

        for mut rect in rects {
            rect.extend_by_items(&self.model);

            let (x, y, width, height) = rect.to_area_extend_ink(&self.model, cell_metrics);
            self.drawing_area.queue_draw_area(x, y, width, height);
        }
    }

    pub fn update_dirty_glyphs(&mut self, render_state: &RenderState) {
        render::shape_dirty(&render_state.font_ctx, &mut self.model, &render_state.hl);
    }
}

impl Grid {
    pub fn new(grid: u64) -> Self {
        let drawing_area = gtk::DrawingArea::new();

        drawing_area.set_can_focus(true);

        drawing_area.add_events(
            (gdk::EventMask::BUTTON_RELEASE_MASK
                | gdk::EventMask::BUTTON_PRESS_MASK
                | gdk::EventMask::BUTTON_MOTION_MASK
                | gdk::EventMask::SCROLL_MASK
                | gdk::EventMask::SMOOTH_SCROLL_MASK
                | gdk::EventMask::ENTER_NOTIFY_MASK
                | gdk::EventMask::LEAVE_NOTIFY_MASK
                | gdk::EventMask::POINTER_MOTION_MASK)
                .bits() as i32,
        );

        Grid {
            grid,
            model: UiModel::empty(),
            drawing_area,
        }
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        self.model.get_cursor()
    }

    pub fn cur_point(&self) -> ModelRect {
        self.model.cur_point()
    }

    pub fn id(&self) -> u64 {
        self.grid
    }

    pub fn resize(&mut self, columns: u64, rows: u64) {
        if self.model.columns != columns as usize || self.model.rows != rows as usize {
            self.model = UiModel::new(rows, columns);
        }
    }

    pub fn cursor_goto(&mut self, row: usize, col: usize) -> ModelRectVec {
        self.model.set_cursor(row, col)
    }

    pub fn clear(&mut self, default_hl: &Rc<Highlight>) {
        self.model.clear(default_hl);
    }

    pub fn line(
        &mut self,
        row: usize,
        col_start: usize,
        cells: Vec<Vec<Value>>,
        highlights: &HighlightMap,
    ) -> ModelRect {
        let mut hl_id = None;
        let mut col_end = col_start;

        for cell in cells {
            let ch = cell.get(0).unwrap().as_str().unwrap_or("");
            hl_id = cell.get(1).and_then(|h| h.as_u64()).or(hl_id);
            let repeat = cell.get(2).and_then(|r| r.as_u64()).unwrap_or(1) as usize;

            self.model.put(
                row,
                col_end,
                ch,
                ch.is_empty(),
                repeat,
                highlights.get(hl_id.unwrap()),
            );
            col_end += repeat;
        }

        ModelRect::new(row, row, col_start, col_end - 1)
    }

    pub fn scroll(
        &mut self,
        top: u64,
        bot: u64,
        left: u64,
        right: u64,
        rows: i64,
        _: i64,
        default_hl: &Rc<Highlight>,
    ) -> ModelRect {
        self.model.scroll(
            top as i64,
            bot as i64 - 1,
            left as usize,
            right as usize - 1,
            rows,
            default_hl,
        )
    }
}

impl Deref for Grid {
    type Target = gtk::DrawingArea;

    fn deref(&self) -> &gtk::DrawingArea {
        &self.drawing_area
    }
}
