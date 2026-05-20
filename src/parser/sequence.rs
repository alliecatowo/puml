#[path = "sequence/identifiers.rs"]
mod sequence_identifiers;
#[path = "sequence/keywords.rs"]
mod sequence_keywords;
#[path = "sequence/messages.rs"]
mod sequence_messages;
#[path = "sequence/participants.rs"]
mod sequence_participants;
#[path = "sequence/syntax.rs"]
mod sequence_syntax;

use sequence_identifiers::{
    clean_ident, extract_class_member_name, extract_component_group_member_name,
};
use sequence_keywords::{note_end_matches, note_kind_from_keyword, parse_keyword, parse_note_head};
use sequence_messages::parse_message;
use sequence_participants::parse_participant;
use sequence_syntax::{
    is_family_common_keyword, is_sequence_keyword, looks_like_arrow_syntax,
    looks_like_virtual_endpoint_syntax, normalize_virtual_endpoint, note_block_continues,
    parse_arrow, split_arrow, split_family_relation_label, split_lifecycle_modifier,
    split_message_label, strip_sequence_arrow_brackets, text_block_continues,
};
