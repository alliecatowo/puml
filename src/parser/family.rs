fn parse_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let (class_visibility, class_line) = strip_class_declaration_visibility(line);
    if let Some(kind) = parse_parenthesized_c4_decl(class_line) {
        return Ok(Some((kind, start)));
    }
    for (keyword, marker) in [
        ("abstract class", Some("<<abstract class>>")),
        ("exception", Some("<<exception>>")),
        ("metaclass", Some("<<metaclass>>")),
        ("stereotype", Some("<<stereotype>>")),
        ("interface", Some("<<interface>>")),
        ("enum", Some("<<enum>>")),
        ("annotation", Some("<<annotation>>")),
        ("protocol", Some("<<protocol>>")),
        ("struct", Some("<<struct>>")),
        ("circle", Some("<<circle>>")),
        ("abstract", Some("<<abstract>>")),
        ("class", None),
    ] {
        let Some(decl) = parse_named_family_decl(class_line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            tags,
            fill_color,
            style_members,
            heritage,
            ..
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_class_visibility_member(&mut members, class_visibility);
        append_family_tag_members(&mut members, tags);
        append_inline_fill_member(&mut members, fill_color);
        append_inline_style_members(&mut members, style_members);
        append_heritage_members(&mut members, heritage);
        return Ok(Some((
            StatementKind::ClassDecl(ClassDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    if let Some(decl) = parse_named_family_decl(class_line, "entity") {
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            tags,
            fill_color,
            style_members,
            heritage,
            ..
        } = decl;
        if has_block || later_lines_contain_ie_family_context(lines, start) {
            let mut members = if has_block {
                let mut members = parse_family_decl_members(lines, start, "entity", &name)?;
                for stereotype in stereotypes.iter().rev() {
                    members.insert(
                        0,
                        ClassMember {
                            text: format!("<<{stereotype}>>"),
                            modifier: None,
                        },
                    );
                }
                members
            } else {
                declaration_marker_members(None, stereotypes)
            };
            append_class_visibility_member(&mut members, class_visibility);
            append_family_tag_members(&mut members, tags);
            append_inline_fill_member(&mut members, fill_color);
            append_inline_style_members(&mut members, style_members);
            append_heritage_members(&mut members, heritage);
            return Ok(Some((
                StatementKind::ClassDecl(ClassDecl {
                    name,
                    alias,
                    members,
                }),
                if has_block {
                    find_family_decl_end(lines, start)
                } else {
                    start
                },
            )));
        }
    }

    for (keyword, marker) in [
        ("diamond", Some("<<diamond>>")),
        ("map", Some("<<map>>")),
        ("object", None),
    ] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            tags,
            fill_color,
            style_members,
            ..
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_family_tag_members(&mut members, tags);
        append_inline_fill_member(&mut members, fill_color);
        append_inline_style_members(&mut members, style_members);
        return Ok(Some((
            StatementKind::ObjectDecl(ObjectDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    if let Some(kind) = parse_association_class_relation(line) {
        return Ok(Some((kind, start)));
    }

    if let Some(decl) = parse_parenthesized_usecase_decl(line) {
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            fill_color,
            style_members,
            business,
            ..
        } = decl;
        let mut members = Vec::new();
        append_business_member(&mut members, business);
        append_inline_fill_member(&mut members, fill_color);
        append_inline_style_members(&mut members, style_members);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    if let Some(decl) = parse_colon_actor_decl(line) {
        let FamilyDeclParts {
            name,
            alias,
            fill_color,
            style_members,
            business,
            ..
        } = decl;
        let mut members = declaration_marker_members(Some("<<actor>>"), Vec::new());
        append_business_member(&mut members, business);
        append_inline_fill_member(&mut members, fill_color);
        append_inline_style_members(&mut members, style_members);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            start,
        )));
    }

    for (keyword, marker) in [
        ("actor/", Some("<<actor>>")),
        ("usecase/", None),
        ("actor", Some("<<actor>>")),
        ("usecase", None),
    ] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            tags,
            fill_color,
            style_members,
            business,
            ..
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_family_tag_members(&mut members, tags);
        append_business_member(&mut members, business || keyword.ends_with('/'));
        append_inline_fill_member(&mut members, fill_color);
        append_inline_style_members(&mut members, style_members);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }
    Ok(None)
}
