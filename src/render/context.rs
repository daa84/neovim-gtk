use std::collections::HashSet;

use pango;

use crate::sys::pango as sys_pango;

use super::itemize::ItemizeIterator;
use crate::ui_model::StyledLine;

pub struct Context {
    font_metrics: FontMetrix,
    font_features: FontFeatures,
    line_space: i32,
}

impl Context {
    pub fn new(pango_context: pango::Context) -> Self {
        Context {
            line_space: 0,
            font_metrics: FontMetrix::new(pango_context, 0),
            font_features: FontFeatures::new(),
        }
    }

    pub fn update(&mut self, pango_context: pango::Context) {
        self.font_metrics = FontMetrix::new(pango_context, self.line_space);
    }

    pub fn update_font_features(&mut self, font_features: FontFeatures) {
        self.font_features = font_features;
    }

    pub fn update_line_space(&mut self, line_space: i32) {
        self.line_space = line_space;
        let pango_context = self.font_metrics.pango_context.clone();
        self.font_metrics = FontMetrix::new(pango_context, self.line_space);
    }

    pub fn itemize(&self, line: &StyledLine) -> Vec<pango::Item> {
        let attr_iter = line.attr_list.get_iterator();

        ItemizeIterator::new(&line.line_str)
            .flat_map(|(offset, len)| {
                pango::itemize(
                    &self.font_metrics.pango_context,
                    &line.line_str,
                    offset as i32,
                    len as i32,
                    &line.attr_list,
                    attr_iter.as_ref(),
                )
            }).collect()
    }

    pub fn create_layout(&self) -> pango::Layout {
        pango::Layout::new(&self.font_metrics.pango_context)
    }

    pub fn font_description(&self) -> &pango::FontDescription {
        &self.font_metrics.font_desc
    }

    pub fn cell_metrics(&self) -> &CellMetrics {
        &self.font_metrics.cell_metrics
    }

    pub fn font_features(&self) -> &FontFeatures {
        &self.font_features
    }

    pub fn font_families(&self) -> HashSet<glib::GString> {
        self.font_metrics
            .pango_context
            .list_families()
            .iter()
            .filter_map(pango::FontFamilyExt::get_name)
            .collect()
    }
}

struct FontMetrix {
    pango_context: pango::Context,
    cell_metrics: CellMetrics,
    font_desc: pango::FontDescription,
}

impl FontMetrix {
    pub fn new(pango_context: pango::Context, line_space: i32) -> Self {
        let font_metrics = pango_context.get_metrics(None, None).unwrap();
        let font_desc = pango_context.get_font_description().unwrap();

        FontMetrix {
            pango_context,
            cell_metrics: CellMetrics::new(&font_metrics, line_space),
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
    fn new(font_metrics: &pango::FontMetrics, line_space: i32) -> Self {
        let ascent = (f64::from(font_metrics.get_ascent()) / f64::from(pango::SCALE)).ceil();
        let descent = (f64::from(font_metrics.get_descent()) / f64::from(pango::SCALE)).ceil();
        let underline_position = (f64::from(font_metrics.get_underline_position()) / f64::from(pango::SCALE)).ceil();
        CellMetrics {
            pango_ascent: font_metrics.get_ascent(),
            pango_descent: font_metrics.get_descent(),
            pango_char_width: font_metrics.get_approximate_char_width(),
            ascent,
            line_height: ascent + descent + f64::from(line_space),
            char_width: f64::from(font_metrics.get_approximate_char_width())
                / f64::from(pango::SCALE),
            underline_position: ascent - underline_position,
            underline_thickness: f64::from(font_metrics.get_underline_thickness()) / f64::from(pango::SCALE),
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

pub struct FontFeatures {
    attr: Option<pango::Attribute>,
}

impl FontFeatures {
    pub fn new() -> Self {
        FontFeatures { attr: None }
    }

    pub fn from(font_features: String) -> Self {
        if font_features.trim().is_empty() {
            return Self::new();
        }

        FontFeatures {
            attr: sys_pango::attribute::new_features(&font_features),
        }
    }

    pub fn insert_into(&self, attr_list: &pango::AttrList) {
        if let Some(ref attr) = self.attr {
            attr_list.insert(attr.clone());
        }
    }
}
