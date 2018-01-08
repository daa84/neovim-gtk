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
    pub fn layout(
        &mut self,
        lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>,
        max_columns: u64,
    ) -> (u64, u64) {
        let rows = ModelLayout::count_lines(&lines, max_columns);

        if rows as usize > self.model.rows || max_columns as usize > self.model.columns {
            let model_cols = ((max_columns / ModelLayout::COLUMNS_STEP) + 1) *
                ModelLayout::COLUMNS_STEP;

            let model_rows = ((rows as u64 / ModelLayout::ROWS_STEP) + 1) * ModelLayout::ROWS_STEP;

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
                    } else {
                        col_idx += 1;
                    }

                    if max_col_idx < col_idx {
                        max_col_idx = col_idx;
                    }

                    self.model.set_cursor(row_idx, col_idx as usize);
                    self.model.put(ch, false, attr.as_ref());
                }
            }
            row_idx += 1;
        }

        (max_col_idx + 1, rows)
    }

    fn count_lines(lines: &Vec<Vec<(Option<Attrs>, Vec<char>)>>, max_columns: u64) -> u64 {
        let mut row_count = 0;

        for line in lines {
            let len: usize = line.iter().map(|c| c.1.len()).sum();
            row_count += len as u64 / (max_columns + 1) + 1;
        }

        row_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines() {
        let lines = vec![vec![(None, vec!['a'; 5])]];

        let rows = ModelLayout::count_lines(&lines, 4);
        assert_eq!(2, rows);
    }
}
