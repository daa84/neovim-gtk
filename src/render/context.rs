use pango;
use pango::prelude::*;

use sys::pango as sys_pango;
use sys::pango::AttrIteratorFactory;

use super::itemize::ItemizeIterator;
use ui_model::StyledLine;

pub struct Context {
    font_metrics: FontMetrix,
    font_features: FontFeatures,
}

impl Context {
    pub fn new(pango_context: pango::Context) -> Self {
        Context {
            font_metrics: FontMetrix::new(pango_context),
            font_features: FontFeatures::new(),
        }
    }

    pub fn update(&mut self, pango_context: pango::Context) {
        self.font_metrics = FontMetrix::new(pango_context);
    }

    pub fn update_font_features(&mut self, font_features: FontFeatures) {
        self.font_features = font_features;
    }

    pub fn itemize(&self, line: &StyledLine) -> Vec<sys_pango::Item> {
        let mut attr_iter = line.attr_list.get_iterator();

        ItemizeIterator::new(&line.line_str)
            .flat_map(|(offset, len)| {
                sys_pango::pango_itemize(
                    &self.font_metrics.pango_context,
                    &line.line_str,
                    offset,
                    len,
                    &line.attr_list,
                    Some(&mut attr_iter),
                )
            })
            .collect()
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
}

struct FontMetrix {
    pango_context: pango::Context,
    cell_metrics: CellMetrics,
    font_desc: pango::FontDescription,
}

impl FontMetrix {
    pub fn new(pango_context: pango::Context) -> Self {
        let font_metrics = pango_context.get_metrics(None, None).unwrap();
        let font_desc = pango_context.get_font_description().unwrap();

        FontMetrix {
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
            line_height: (font_metrics.get_ascent() + font_metrics.get_descent()) as f64
                / pango::SCALE as f64,
            char_width: font_metrics.get_approximate_digit_width() as f64 / pango::SCALE as f64,
            underline_position: (font_metrics.get_ascent() - font_metrics.get_underline_position())
                as f64 / pango::SCALE as f64,
            underline_thickness: font_metrics.get_underline_thickness() as f64
                / pango::SCALE as f64,
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

    pub fn insert_attr(&self, attr_list: &pango::AttrList) {
        if let Some(ref attr) = self.attr {
            attr_list.insert(attr.clone());
        }
    }
}
