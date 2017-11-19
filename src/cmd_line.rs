use std::collections::HashMap;

use gtk;
use gtk::prelude::*;

use neovim_lib::Value;

use ui_model::{UiModel, Attrs};

pub struct Level {
    model: UiModel,
}

impl Level {
    const COLUMNS_STEP: u64 = 50;
    const ROWS_STEP: u64 = 10;

    pub fn from(
        content: Vec<(HashMap<String, Value>, String)>,
        pos: u64,
        firstc: String,
        prompt: String,
        indent: u64,
        level: u64,
    ) -> Self {
        //TODO: double width chars

        let prompt = prompt_lines(firstc, prompt, indent);
        let content: Vec<(Attrs, Vec<char>)> = content
            .iter()
            .map(|c| (Attrs::from_value_map(&c.0), c.1.chars().collect()))
            .collect();

        let width = (content.iter().map(|c| c.1.len()).count() + prompt.last().map_or(0, |p| p.len())) as u64;
        let columns = ((width / Level::COLUMNS_STEP) + 1) * Level::COLUMNS_STEP;
        let rows = ((prompt.len() as u64 / Level::ROWS_STEP) + 1) * Level::ROWS_STEP;

        let model = UiModel::new(rows, columns);

        Level { model }
    }
}

fn prompt_lines(firstc: String, prompt: String, indent: u64) -> Vec<Vec<char>> {
    if !firstc.is_empty() {
        vec![firstc.chars().chain((0..indent).map(|_| ' ')).collect()]
    } else if !prompt.is_empty() {
        prompt.lines().map(|l| l.chars().collect()).collect()
    } else {
        vec![]
    }
}

pub struct CmdLine {
    popover: gtk::Popover,
    levels: Vec<Level>,
}

impl CmdLine {
    pub fn new(drawing: &gtk::DrawingArea) -> Self {
        let popover = gtk::Popover::new(Some(drawing));
        popover.set_modal(false);
        let edit_frame = gtk::Frame::new(None);
        edit_frame.set_shadow_type(gtk::ShadowType::In);
        let drawing_area = gtk::DrawingArea::new();
        edit_frame.add(&drawing_area);
        edit_frame.show_all();

        popover.add(&edit_frame);

        CmdLine {
            levels: Vec::new(),
            popover,
        }
    }

    pub fn show_level(&mut self, level: Level) {
        self.levels.push(level);
        self.popover.popup();
    }
}
