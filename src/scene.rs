#[derive(Debug, Clone)]
pub struct Scene {
    pub width: i32,
    pub height: i32,
    pub title: Option<Label>,
    pub participants: Vec<ParticipantBox>,
    pub lifelines: Vec<Lifeline>,
    pub messages: Vec<MessageLine>,
    pub notes: Vec<NoteBox>,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub x: i32,
    pub y: i32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ParticipantBox {
    pub id: String,
    pub display: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct Lifeline {
    pub participant_id: String,
    pub x: i32,
    pub y1: i32,
    pub y2: i32,
}

#[derive(Debug, Clone)]
pub struct MessageLine {
    pub from_id: String,
    pub to_id: String,
    pub x1: i32,
    pub y: i32,
    pub x2: i32,
    pub arrow: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NoteBox {
    pub target_id: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: String,
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub margin: i32,
    pub participant_spacing: i32,
    pub participant_width: i32,
    pub participant_height: i32,
    pub title_height: i32,
    pub message_row_height: i32,
    pub note_width: i32,
    pub note_padding: i32,
    pub footer_height: i32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            margin: 24,
            participant_spacing: 160,
            participant_width: 120,
            participant_height: 32,
            title_height: 28,
            message_row_height: 40,
            note_width: 180,
            note_padding: 8,
            footer_height: 24,
        }
    }
}
