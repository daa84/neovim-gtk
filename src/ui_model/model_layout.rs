use ui_model::{Attrs, UiModel};

pub struct ModelLayout {
    pub model: UiModel,
    rows_filled: usize,
}

impl ModelLayout {
    const COLUMNS_STEP: usize = 50;
    const ROWS_STEP: usize = 10;

    pub fn new() -> Self {
        ModelLayout {
            model: UiModel::new(ModelLayout::ROWS_STEP as u64, ModelLayout::COLUMNS_STEP as u64),
            rows_filled: 0,
        }
    }

    fn check_model_size(&mut self, rows: usize, columns: usize) {
        if rows > self.model.rows || columns > self.model.columns {
            let model_cols =
                ((columns / ModelLayout::COLUMNS_STEP) + 1) * ModelLayout::COLUMNS_STEP;

            let model_rows = ((rows / ModelLayout::ROWS_STEP) + 1) * ModelLayout::ROWS_STEP;

            let mut model = UiModel::new(model_rows as u64, model_cols as u64);
            self.model.copy_rows(&mut model, self.rows_filled);
        }
    }

    /// Wrap all lines into model
    ///
    /// returns actual width
    pub fn layout(
        &mut self,
        lines: Vec<Vec<(Option<Attrs>, Vec<char>)>>,
        max_columns: usize,
    ) -> (usize, usize) {
        let rows = ModelLayout::count_lines(&lines, max_columns);

        self.check_model_size(rows, max_columns);
        self.rows_filled = rows;

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

    fn count_lines(lines: &Vec<Vec<(Option<Attrs>, Vec<char>)>>, max_columns: usize) -> usize {
        let mut row_count = 0;

        for line in lines {
            let len: usize = line.iter().map(|c| c.1.len()).sum();
            row_count += len / (max_columns + 1) + 1;
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
