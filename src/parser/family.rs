#[path = "family/declarations.rs"]
mod family_declarations;
#[path = "family/relations.rs"]
mod family_relations;
#[path = "family/scoping.rs"]
mod family_scoping;

use family_declarations::{
    append_inline_fill_member, declaration_marker_members, find_family_decl_end,
    later_lines_contain_class_family_declaration, later_lines_contain_sequence_family_keywords,
    later_lines_contain_usecase_family_declaration, parse_class_member, parse_family_decl_members,
    parse_family_declaration, parse_named_family_decl, parse_parenthesized_usecase_decl,
    split_declaration_inline_fill, strip_declaration_stereotypes, FamilyDeclParts,
};
use family_relations::{
    clean_bracketed_ident, parse_family_member_row, parse_family_relation,
    parse_relation_color_token, split_family_arrow, split_family_arrow_styled,
};
use family_scoping::{
    group_body_contains_class_family, group_body_contains_component_family,
    group_body_contains_object_family, group_body_contains_usecase_family,
    parse_class_scoping_block, qualify_scoped_identifier, qualify_scoped_relation,
    scoped_family_kind_for_block, ScopedGroupContent,
};
