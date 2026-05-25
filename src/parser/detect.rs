fn detect_non_sequence_family(line: &str) -> Option<DiagramKind> {
    if line.eq_ignore_ascii_case("stdlib") {
        return Some(DiagramKind::Stdlib);
    }

    if line.starts_with("relationship ") {
        return Some(DiagramKind::Chen);
    }

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
        || line.starts_with("container ")
        || line.starts_with("actor ")
        // `[Name]` shorthand for component — not [*]/[H]/[H*] pseudo-states
        || (line.starts_with('[')
            && !line.starts_with("[*]")
            && !line.starts_with("[H]")
            && !line.starts_with("[H*]"))
    {
        return Some(DiagramKind::Component);
    }

    if line.starts_with("node ")
        || line.starts_with("action ")
        || line.starts_with("agent ")
        || line.starts_with("artifact ")
        || line.starts_with("actor/ ")
        || line.starts_with("boundary ")
        || line.starts_with("cloud ")
        || line.starts_with("circle ")
        || line.starts_with("collections ")
        || line.starts_with("control ")
        || line.starts_with("entity ")
        || line.starts_with("frame ")
        || line.starts_with("hexagon ")
        || line.starts_with("label ")
        || line.starts_with("person ")
        || line.starts_with("process ")
        || line.starts_with("queue ")
        || line.starts_with("stack ")
        || line.starts_with("storage ")
        || line.starts_with("database ")
        || line.starts_with("usecase/ ")
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
        || line.starts_with("analog ")
        || line.starts_with("compact robust ")
        || line.starts_with("compact concise ")
        || line.starts_with("compact clock ")
        || line.starts_with("compact binary ")
        || line.starts_with("compact analog ")
        || line == "hide time-axis"
        || line == "mode compact"
        || line == "manual time-axis"
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

    if line.starts_with("component ")
        || line.starts_with("vspace ")
        || line.starts_with("move ")
        || line.starts_with("goto ")
        || line.starts_with("print(")
        || line.starts_with('$')
    {
        return Some(DiagramKind::Wire);
    }

    None
}
