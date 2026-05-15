use crate::scene::TextOverflowPolicy;

#[derive(Debug, Clone)]
pub struct Theme {
    pub skinparams: Vec<(String, String)>,
    pub footbox_visible: bool,
    pub text_overflow_policy: TextOverflowPolicy,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: false,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}

impl Theme {
    pub fn new() -> Self {
        Self {
            skinparams: Vec::new(),
            footbox_visible: true,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}
