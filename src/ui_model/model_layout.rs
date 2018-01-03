use ui_model::{UiModel, Attrs};

pub struct ModelLayout {
    pub model: UiModel,
}

impl ModelLayout {
    const COLUMNS_STEP: u64 = 50;
    const ROWS_STEP: u64 = 10;

    pub fn new() -> Self {
        ModelLayout { model: UiModel::new(ModelLayout::ROWS_STEP, ModelLayout::COLUMNS_STEP) }
    }

    /// Wrap all lines into model
    ///
    /// returns actual width
    pub fn layout(&mut self, lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>, max_columns: u64) -> (u64, u64) {
        //FIXME: lines.len is not real lines count
        if lines.len() > self.model.rows || max_columns as usize > self.model.columns {
            let model_cols = ((max_columns / ModelLayout::COLUMNS_STEP) + 1) *
                ModelLayout::COLUMNS_STEP;
            let model_rows = ((lines.len() as u64 / ModelLayout::ROWS_STEP) + 1) *
                ModelLayout::ROWS_STEP;

            self.model = UiModel::new(model_rows, model_cols);
        }


        let mut max_col_idx = 0;
        let mut col_idx = 0;
        let mut row_idx = 0;
        for content in lines {
            for (attr, ch_list) in content {
                for ch in ch_list {
                    if col_idx >= max_columns {
                        col_idx = 0;
                        row_idx += 1;
                    }

                    if max_col_idx < col_idx {
                        max_col_idx = col_idx;
                    }

                    self.model.set_cursor(row_idx, col_idx as usize);
                    self.model.put(ch, false, attr.as_ref());
                    col_idx += 1;
                }
            }
            row_idx += 1;
        }

        (max_col_idx + 1, row_idx as u64)
    }
}
