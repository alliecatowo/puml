fn detect_non_sequence_family(line: &str) -> Option<DiagramKind> {
    if line.starts_with("component ")
        || line.starts_with("interface ")
        || line.starts_with("port ")
        || line.starts_with("portin ")
        || line.starts_with("portout ")
        || line.starts_with("package ")
        || line.starts_with("rectangle ")
        || line.starts_with("folder ")
        || line.starts_with("file ")
        || line.starts_with("card ")
        || line.starts_with("actor ")
    {
        return Some(DiagramKind::Component);
    }

    if line.starts_with("node ")
        || line.starts_with("artifact ")
        || line.starts_with("cloud ")
        || line.starts_with("frame ")
        || line.starts_with("storage ")
        || line.starts_with("database ")
    {
        return Some(DiagramKind::Deployment);
    }

    if line.starts_with("state ") || line == "[*]" || line == "[H]" || line == "[H*]" {
        return Some(DiagramKind::State);
    }
    // State transitions involving pseudo-states
    if (line.starts_with("[*]") || line.starts_with("[H]") || line.starts_with("[H*]"))
        && line.contains("-->")
    {
        return Some(DiagramKind::State);
    }
    // Any line that is `X --> Y` where Y is `[*]`, `[H]`, or `[H*]`
    if line.contains("-->") {
        if let Some(idx) = line.find("-->") {
            let rhs = line[idx + 3..].trim();
            // Strip label part
            let rhs_base = rhs.split(':').next().unwrap_or(rhs).trim();
            if matches!(rhs_base, "[*]" | "[H]" | "[H*]") {
                return Some(DiagramKind::State);
            }
        }
    }

    if line.starts_with('*')
        || line.starts_with('+')
        || line.starts_with('-')
        || line.starts_with('#')
    {
        return Some(DiagramKind::MindMap);
    }

    if line.starts_with("wbs ") {
        return Some(DiagramKind::Wbs);
    }

    if line.starts_with("start")
        || line.starts_with("stop")
        || line.starts_with(':')
        || line.starts_with("(*)")
        || line.starts_with("if ")
        || line.starts_with("elseif ")
        || line == "else"
        || line.starts_with("endif")
        || line.starts_with("switch ")
        || line.starts_with("case ")
        || line.starts_with("endswitch")
        || line.starts_with("repeat")
        || line.starts_with("while ")
        || line.starts_with("fork")
        || line.starts_with("split")
        || line.starts_with("end split")
        || line.starts_with("kill")
        || line.starts_with("break")
        || line.starts_with("continue")
        || line.starts_with("label ")
        || line.starts_with("goto ")
        || line.starts_with("backward")
        || line.starts_with("partition ")
        || line.starts_with("swimlane ")
        || line.starts_with('|')
        || line.starts_with("detach")
    {
        return Some(DiagramKind::Activity);
    }

    if line.starts_with("robust ")
        || line.starts_with("concise ")
        || line.starts_with("clock ")
        || line.starts_with("binary ")
        || line.starts_with('@')
        // Timing-specific scale syntax: "scale N as N" (maps clock units to pixels).
        // Plain "scale 1.5" / "scale 800*600" / "scale max N" is the output-scale
        // directive and should not be classified as a timing diagram.
        || (line.starts_with("scale ") && line.contains(" as "))
    {
        return Some(DiagramKind::Timing);
    }
    if line.starts_with("salt ") {
        return Some(DiagramKind::Salt);
    }

    if line.starts_with("salt ") {
        return Some(DiagramKind::Salt);
    }

    None
}
