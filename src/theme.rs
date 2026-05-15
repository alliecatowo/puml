#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub skinparams: Vec<(String, String)>,
    pub footbox_visible: bool,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: true,
        }
    }
}
