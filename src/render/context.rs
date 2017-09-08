use pangocairo::FontMap;
use pango::prelude::*;
use pango;

use sys::pango as sys_pango;

use ui_model::StyledLine;

pub struct Context {
    state: ContextState,
}

impl Context {
    pub fn new(font_desc: pango::FontDescription) -> Self {
        Context { state: ContextState::new(font_desc) }
    }

    pub fn update(&mut self, font_desc: pango::FontDescription) {
        self.state = ContextState::new(font_desc);
    }

    pub fn itemize(&self, line: &StyledLine) -> Vec<sys_pango::Item> {
        sys_pango::pango_itemize(
            &self.state.pango_context,
            line.line_str.trim_right(),
            &line.attr_list,
        )
    }

    #[inline]
    pub fn font_description(&self) -> &pango::FontDescription {
        &self.state.font_desc
    }

    #[inline]
    pub fn ascent(&self) -> f64 {
        self.state.cell_metrics.ascent
    }

    #[inline]
    pub fn cell_metrics(&self) -> &CellMetrics {
        &self.state.cell_metrics
    }
}

struct ContextState {
    pango_context: pango::Context,
    cell_metrics: CellMetrics,
    font_desc: pango::FontDescription,
}

impl ContextState {
    pub fn new(font_desc: pango::FontDescription) -> Self {
        let font_map = FontMap::get_default();
        let pango_context = font_map.create_context().unwrap();
        pango_context.set_font_description(&font_desc);

        let font_metrics = pango_context.get_metrics(None, None).unwrap();

        ContextState {
            pango_context,
            cell_metrics: CellMetrics::new(&font_metrics),
            font_desc,
        }
    }
}

pub struct CellMetrics {
    pub line_height: f64,
    pub char_width: f64,
    pub ascent: f64,
    pub pango_ascent: i32,
    pub pango_descent: i32,
    pub pango_char_width: i32,
}

impl CellMetrics {
    fn new(font_metrics: &pango::FontMetrics) -> Self {
        CellMetrics {
            pango_ascent: font_metrics.get_ascent(),
            pango_descent: font_metrics.get_descent(),
            pango_char_width: font_metrics.get_approximate_digit_width(),
            ascent: font_metrics.get_ascent() as f64 / pango::SCALE as f64,
            line_height: (font_metrics.get_ascent() + font_metrics.get_descent()) as f64 /
                pango::SCALE as f64,
            char_width: font_metrics.get_approximate_digit_width() as f64 / pango::SCALE as f64,
        }
    }
}
