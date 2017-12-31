use pangocairo::FontMap;
use pango::prelude::*;
use pango;

use sys::pango as sys_pango;
use sys::pango::AttrIteratorFactory;

use ui_model::StyledLine;
use super::itemize::ItemizeIterator;

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
        let mut attr_iter = line.attr_list.get_iterator();

        ItemizeIterator::new(&line.line_str)
            .flat_map(|(offset, len)| {
                sys_pango::pango_itemize(
                    &self.state.pango_context,
                    &line.line_str,
                    offset,
                    len,
                    &line.attr_list,
                    Some(&mut attr_iter),
                )
            })
            .collect()
    }

    #[inline]
    pub fn font_description(&self) -> &pango::FontDescription {
        &self.state.font_desc
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
        let font_map = FontMap::get_default().unwrap();
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
    pub underline_position: f64,
    pub underline_thickness: f64,
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
            underline_position: (font_metrics.get_ascent() -
                                     font_metrics.get_underline_position()) as
                f64 / pango::SCALE as f64,
            underline_thickness: font_metrics.get_underline_thickness() as f64 /
                pango::SCALE as f64,
        }
    }

    #[cfg(test)]
    pub fn new_hw(line_height: f64, char_width: f64) -> Self {
        CellMetrics {
            pango_ascent: 0,
            pango_descent: 0,
            pango_char_width: 0,
            ascent: 0.0,
            line_height,
            char_width,
            underline_position: 0.0,
            underline_thickness: 0.0,
        }
    }
}
