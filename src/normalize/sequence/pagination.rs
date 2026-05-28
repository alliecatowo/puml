use super::*;

pub(super) fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {
    let mut pages = Vec::new();
    let mut page_events = Vec::new();
    let mut current_title = document.title.clone();

    for event in &document.events {
        if let SequenceEventKind::NewPage(next_title) = &event.kind {
            pages.push(page_from(document, &page_events, current_title.clone()));
            page_events.clear();
            current_title = cleaned_title(next_title).or_else(|| document.title.clone());
            continue;
        }
        page_events.push(event.clone());
    }

    pages.push(page_from(document, &page_events, current_title));
    pages
}

fn page_from(
    document: &SequenceDocument,
    events: &[SequenceEvent],
    title: Option<String>,
) -> SequencePage {
    SequencePage {
        participants: document.participants.clone(),
        participant_groups: document.participant_groups.clone(),
        events: events.to_vec(),
        teoz: document.teoz,
        title,
        header: document.header.clone(),
        header_align: document.header_align,
        footer: document.footer.clone(),
        footer_align: document.footer_align,
        caption: document.caption.clone(),
        legend: document.legend.clone(),
        skinparams: document.skinparams.clone(),
        style: document.style.clone(),
        footbox_visible: document.footbox_visible,
        scale: document.scale.clone(),
        legend_halign: document.legend_halign,
        legend_valign: document.legend_valign,
        warnings: document.warnings.clone(),
        hide_unlinked: document.hide_unlinked,
        hidden_participants: document.hidden_participants.clone(),
        sprites: document.sprites.clone(),
        list_sprites: document.list_sprites,
        mainframe: document.mainframe.clone(),
        created_participants: document.created_participants.clone(),
    }
}

fn cleaned_title(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}
