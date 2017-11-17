use ui_model;

pub struct CmdLine {
    content: Vec<(ui_model::Attrs, String)>,
    pos: u64,
    firstc: String,
    prompt: String,
    indent: u64,
    level: u64,
}

impl CmdLine {
    pub fn new(
        content: Vec<(ui_model::Attrs, String)>,
        pos: u64,
        firstc: String,
        prompt: String,
        indent: u64,
        level: u64,
    ) -> Self {
        CmdLine {
            content,
            pos,
            firstc,
            prompt,
            indent,
            level,
        }
    }
}
