use ui_model::{Attrs, UiModel};

pub struct ModelLayout {
    pub model: UiModel,
    rows_filled: usize,
}

impl ModelLayout {
    const ROWS_STEP: usize = 10;

    pub fn new(columns: u64) -> Self {
        ModelLayout {
            model: UiModel::new(ModelLayout::ROWS_STEP as u64, columns),
            rows_filled: 0,
        }
    }

    pub fn layout_append(&mut self, lines: &Vec<Vec<(Option<Attrs>, Vec<char>)>>) -> (usize, usize) {
        let rows_filled = self.rows_filled;
        self.layout_replace(rows_filled, lines)
    }

    pub fn layout(&mut self,
        lines: &Vec<Vec<(Option<Attrs>, Vec<char>)>>,
        ) -> (usize, usize) {
        self.layout_replace(0, lines)
    }

    fn check_model_size(&mut self, rows: usize) {
        if rows > self.model.rows {
            let model_cols = self.model.columns;
            let model_rows = ((rows / (ModelLayout::ROWS_STEP + 1)) + 1) * ModelLayout::ROWS_STEP;

            let mut model = UiModel::new(model_rows as u64, model_cols as u64);
            self.model.copy_rows(&mut model, self.rows_filled - 1);
            self.model = model;
        }
    }

    /// Wrap all lines into model
    ///
    /// returns actual width
    fn layout_replace(
        &mut self,
        row_offset: usize,
        lines: &Vec<Vec<(Option<Attrs>, Vec<char>)>>,
    ) -> (usize, usize) {
        let rows = ModelLayout::count_lines(&lines, self.model.columns);

        self.check_model_size(rows + row_offset);
        self.rows_filled = rows + row_offset;

        let mut max_col_idx = 0;
        let mut col_idx = 0;
        let mut row_idx = row_offset;
        for content in lines {
            for &(ref attr, ref ch_list) in content {
                for ch in ch_list {
                    if col_idx >= self.model.columns {
                        col_idx = 0;
                        row_idx += 1;
                    }

                    self.model.set_cursor(row_idx, col_idx as usize);
                    self.model.put(*ch, false, attr.as_ref());

                    if max_col_idx < col_idx {
                        max_col_idx = col_idx;
                    }

                    col_idx += 1;
                }

                if col_idx < self.model.columns {
                    self.model.model[row_idx].clear(col_idx, self.model.columns - 1);
                }
            }
            row_idx += 1;
        }

        (max_col_idx + 1, self.rows_filled)
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

    #[test]
    fn test_resize() {
        let lines = vec![vec![(None, vec!['a'; 5])]; ModelLayout::ROWS_STEP];
        let mut model = ModelLayout::new(5);

        let (cols, rows) = model.layout(&lines);
        assert_eq!(5, cols);
        assert_eq!(ModelLayout::ROWS_STEP, rows);

        let (cols, rows) = model.layout_append(&lines);
        assert_eq!(5, cols);
        assert_eq!(ModelLayout::ROWS_STEP * 2, rows);
        assert_eq!(ModelLayout::ROWS_STEP * 2, model.model.rows);
    }
}
