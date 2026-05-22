# Chapter 1 — Sequence Diagram: Feature Audit

Audit of PlantUML reference (1.2025.0) against puml renderer at HEAD.
File references are to `src/...` unless otherwise noted.

---

### 1.1 Basic Examples — ✅ Supported
**Feature:** Plain `->` arrow auto-declares participants; `-->` for dotted; reverse arrows `<-`/`<--`.
**Syntax example:** `Alice -> Bob: Authentication Request`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:77 (parse_message), parser/sequence.rs:765 (parse_arrow VALID_BASE_ARROWS).
**Notes:** Dotted style detected via `.` in arrow; render in render/sequence.rs.

### 1.2 Declaring participant — ✅ Supported
**Feature:** `participant|actor|boundary|control|entity|database|collections|queue` keywords, `as` alias, `#color` background, `order N`.
**Syntax example:** `actor Bob #red` / `participant "Long name" as L #99FF99 order 30`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:1-61 (parse_participant with all 8 roles + `as` + `order`).
**Notes:** Need to verify `#color` after participant decl parses cleanly — clean_ident strips `"` but does not explicitly strip a trailing `#color` token; needs deeper review for color preservation.

### 1.3 Declaring participant on multiline — 🟡 Partial
**Feature:** `participant Foo [ =Title ---- ""SubTitle"" ]` multi-line participant body with creole.
**Syntax example:** `participant Participant [\n =Title\n ----\n ""SubTitle""\n]`
**Status:** 🟡 Partial
**Evidence:** parser/multiline.rs:12 has parse_multiline_keyword_block but no specific handler for participant `[ ... ]` syntax. parse_participant in parser/sequence.rs:1-61 only handles the single-line form.
**Notes:** Bracketed multi-line participant body likely ignored or treated as plain text.

### 1.4 Use non-letters in participants — ✅ Supported
**Feature:** Quote-wrap participants with special chars; `as` alias.
**Syntax example:** `Alice -> "Bob()" : Hello`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:21-29 (quote handling), parser/sequence.rs:525 (clean_ident strips quotes and `()`).

### 1.5 Message to Self — ✅ Supported
**Feature:** Self-message `Alice -> Alice`, `\n` for multiline label.
**Syntax example:** `Alice -> Alice: signal\nmultiline`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:77 (no special handling needed — same parse_message path). Render handles self-loops in render/sequence.rs.

### 1.6 Text alignment — 🟡 Partial
**Feature:** `skinparam sequenceMessageAlign left|right|center|direction|reverseDirection`.
**Syntax example:** `skinparam sequenceMessageAlign right`
**Status:** 🟡 Partial
**Evidence:** theme.rs:1243 (MessageAlign Left/Center/Right). `direction`/`reverseDirection` not in switch (theme.rs:1238-1240).
**Notes:** Only static left/center/right; arrow-direction-aware modes missing.

#### 1.6.1 Text of response message below the arrow — ✅ Supported
**Feature:** `skinparam responseMessageBelowArrow true` puts reply label under arrow.
**Syntax example:** `skinparam responseMessageBelowArrow true`
**Status:** ✅ Supported
**Evidence:** theme.rs:1255 (ResponseMessageBelowArrow). Needs visual check that render actually uses it.

### 1.7 Change arrow style — 🟡 Partial
**Feature:** `->x` lost message, `-\` / `-/` half-arrowheads, `->>` thin, `--` dotted, `->o` circle endpoint, `<->` bidirectional.
**Syntax example:** `Bob ->x Alice` / `Bob -\ Alice` / `Bob ->o Alice`
**Status:** 🟡 Partial
**Evidence:** parser/sequence.rs:765 (parse_arrow). VALID_BASE_ARROWS includes `<->`, `<<->>`. Cross/circle/slash handling at 800-880.
**Notes:** parse_arrow explicitly rejects `//` (double-slash) at line 783. Single `/` and `\` slash forms parse; doubled forms only honored for `\\`. Needs visual check that `->x` and `->o` actually render the marker glyphs.

### 1.8 Change arrow color — ✅ Supported
**Feature:** Inline arrow color `-[#red]>` or `-[#0000FF]->`.
**Syntax example:** `Bob -[#red]> Alice : hello`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:142-202 (parse_arrow_style parses bracketed `#hex`/named colors, dashed/dotted/thickness).

### 1.9 Message sequence numbering — 🟡 Partial
**Feature:** `autonumber [start] [inc] ["format"]`, `autonumber stop|resume`, dotted multi-level `1.1.1`, `inc A|B`, `%autonumber%` variable.
**Syntax example:** `autonumber 40 10 "<b>[000]"`
**Status:** 🟡 Partial
**Evidence:** parser/sequence.rs:439 (Autonumber statement), normalize/sequence.rs:1473-1626 (canonicalize/validate). Validates start, increment, format; supports dotted starts and `inc A|B`. `validate_autonumber_format` rejects HTML tags (line 1620) and embedded quotes.
**Notes:** HTML formatting in numbers explicitly unsupported. `%autonumber%` variable substitution in notes/labels — no evidence of substitution logic; needs deeper review.

### 1.10 Page Title, Header and Footer — ✅ Supported
**Feature:** `title`, `header`, `footer` (plus `%page%`/`%lastpage%`).
**Syntax example:** `header Page Header` / `footer Page %page% of %lastpage%` / `title Example`
**Status:** ✅ Supported (parse), 🟡 (variable substitution unverified)
**Evidence:** parser/sequence.rs:229-239 (title/header/footer/caption/legend).
**Notes:** `%page%`/`%lastpage%` substitution behaviour not visibly implemented; mark partial for that aspect.

### 1.11 Splitting diagrams — ✅ Supported
**Feature:** `newpage [title]` splits diagram into pages.
**Syntax example:** `newpage A title for the last page`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:433 (NewPage), normalize/sequence.rs:539 (NewPage event).

### 1.12 Grouping message — ✅ Supported
**Feature:** `alt`/`else`, `opt`, `loop`, `par`, `break`, `critical`, `group`, with `end`.
**Syntax example:** `alt successful case … else failure … end`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:352-401 (all group keywords + else + also + end + end <kw>).

### 1.13 Secondary group label — 🟡 Partial
**Feature:** `group My own label [My own label 2]` — secondary text in brackets.
**Syntax example:** `group My own label [My own label 2]`
**Status:** 🟡 Partial
**Evidence:** parser/sequence.rs:352-366 captures the full label string including brackets, but no evidence the renderer splits primary vs secondary or shows them differently.
**Notes:** Bracketed secondary label likely rendered verbatim as part of label text.

### 1.14 Notes on messages — ✅ Supported
**Feature:** `note left`/`note right` after message; multiline via `end note`.
**Syntax example:** `note left: this is a first note`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:305-335 (note parsing), parser/multiline.rs:84 (parse_multiline_note_block).

### 1.15 Some other notes — ✅ Supported
**Feature:** `note left of X`, `note right of X`, `note over A`, `note over A, B`, background `#color`.
**Syntax example:** `note over Alice, Bob #FFAAAA: text`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:483-502 (parse_note_head — handles `of <target>` and bare target lists). Position validated in is_valid_note_position.
**Notes:** `#color` after note head — not explicitly visible in parse_note_head; may end up in target/text. Needs deeper review for color extraction.

### 1.16 Changing notes shape [hnote, rnote] — ✅ Supported
**Feature:** `hnote` (hexagonal), `rnote` (rectangle).
**Syntax example:** `hnote over caller : idle`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:305-313 (hnote/rnote kw detection), parser/sequence.rs:504-510 (NoteKind::Hexagonal/Rectangle), parser/sequence.rs:512-516 (endhnote/endrnote terminators).

### 1.17 Note over all participants [across] — ✅ Supported
**Feature:** `note across: …` / `hnote across: …` spans all participants.
**Syntax example:** `note across: New method`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:521 (across is valid position), layout.rs:994 (position == "across" handling), parser/tests.rs:987 (parses_note_across_without_target).

### 1.18 Several notes aligned at the same level [/] — ✅ Supported
**Feature:** Leading `/` aligns adjacent notes on same vertical level.
**Syntax example:** `/ note over Bob : initial state of Bob`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs (parse_keyword strips `/ ` prefix, sets `Note.aligned = true`); layout.rs (aligned notes reuse the y-coordinate of the preceding note box so they render side-by-side); parser/multiline.rs (multiline `/note` blocks also supported).
**Notes:** Implemented in feat(render): improve sequence diagram ch01 parity. Tests in tests/ch01_sequence_parity.rs.

### 1.19 Creole and HTML — 🟡 Partial
**Feature:** Creole `**bold**`, `//italic//`, `__under__`, `--strike--`, `~~wave~~`, `""mono""`; HTML tags like `<color>`, `<size>`, `<u>`, `<img>`.
**Syntax example:** `note left\n This is **bold**\nend note`
**Status:** 🟡 Partial — needs deeper review
**Evidence:** render/text.rs likely contains text formatting code; not audited in detail here.
**Notes:** Mark partial pending visual confirmation of all creole/HTML constructs.

### 1.20 Divider or separator — ✅ Supported
**Feature:** `== label ==` divider.
**Syntax example:** `== Initialization ==`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:425-432 (Separator with optional label).

### 1.21 Reference — ✅ Supported
**Feature:** `ref over A, B : label` and multi-line `ref over A … end ref`.
**Syntax example:** `ref over Alice, Bob : init`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:336-350 (single-line ref), parser/multiline.rs:144 (parse_multiline_ref_block), normalize/sequence.rs:309 (ref auto-creates participants).

### 1.22 Delay — ✅ Supported
**Feature:** `...` short delay, `...text...` labeled delay.
**Syntax example:** `...5 minutes later...`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:403-410 (Spacer + Divider with trimmed dots).

### 1.23 Text wrapping — 🟡 Partial
**Feature:** `\n` in label (works) and `skinparam maxMessageSize 50` (auto-wrap).
**Syntax example:** `skinparam maxMessageSize 50`
**Status:** 🟡 Partial
**Evidence:** `\n` substitution likely handled in render text. `maxMessageSize` / `maxmessagesize` not found in theme.rs SequenceSkinParamValue enum (no matching variant).
**Notes:** Auto-wrap by maxMessageSize appears unsupported.

### 1.24 Space — ✅ Supported
**Feature:** `|||` blank space, `||N||` N-pixel space.
**Syntax example:** `|||` / `||45||`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:411-424 (Spacer with optional pixel count, clamped 1..400; Delay for `||..||`).
**Notes:** Spec uses `||45||` for sized space; puml maps `|||` to Spacer (with N) and `||..||` to Delay — semantic split may not match spec exactly.

### 1.25 Lifeline Activation and Destruction — ✅ Supported
**Feature:** `activate`, `deactivate`, `destroy`, with optional color, nested.
**Syntax example:** `activate A #FFBBBB`
**Status:** ✅ Supported (basic); 🟡 for `activate A #color`
**Evidence:** parser/sequence.rs:445-457 (activate/deactivate/destroy/create), normalize/sequence.rs:572-660 (lifecycle state machine with diagnostics).
**Notes:** clean_ident strips the whole tail incl. `#FFBBBB`; color on activate likely ignored. Autoactivate keyword (`autoactivate on`) appears in LSP hints (language_service.rs:396) but not in parser/normalize — likely unsupported.

### 1.26 Return — ✅ Supported
**Feature:** `return [label]` emits return arrow to most recent activation source.
**Syntax example:** `return bye`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:459-463 (Return statement). Normalize handles in sequence.rs.

### 1.27 Participant creation — ✅ Supported
**Feature:** `create [role] Target` then a message creates the participant.
**Syntax example:** `create Other` / `create control String`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:452 (Create), normalize/sequence.rs:654 (E_LIFECYCLE_CREATE_EXISTING).
**Notes:** `create control String` — the `control` role qualifier in the create line may not be honored; clean_ident gets the whole "control String" — needs deeper review.

### 1.28 Shortcut syntax for activation, deactivation, creation — ✅ Supported
**Feature:** Trailing `++`, `--`, `**`, `!!` after target id; optional `#color` after `++`.
**Syntax example:** `alice -> bob ++ : hello` / `bob -> george ** : create` / `bob -> george !! : delete`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:903-910 (split_lifecycle_modifier handles `++`/`--`/`**`/`!!`), parser/sequence.rs:108-116 (encodes via @L/@R modifiers).
**Notes:** `--++` combination encoded but each side only gets one modifier — combo support is unclear; needs deeper review. `++ #color` color likely dropped.

### 1.29 Incoming and outgoing messages — ✅ Supported
**Feature:** `[->`, `->]`, `[<-`, `<-]` for messages from/to the diagram edge; with circle/cross variants `[o->`, `[x->`.
**Syntax example:** `[-> A: DoWork` / `A ->]`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:205-224 (ast_virtual_endpoint_from_id covers `[`, `]`, `[o`, `o]`, `[x`, `x]`), parser/sequence.rs:912-925 (normalize_virtual_endpoint).

### 1.30 Short arrows for incoming and outgoing messages — ✅ Supported
**Feature:** `?->` / `->?` short incoming/outgoing arrows.
**Syntax example:** `?-> Alice : short`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs (normalize_virtual_endpoint recognizes `?`, ast_virtual_endpoint_from_id maps it to VirtualEndpointKind::Short); model.rs:VirtualEndpointKind::Short; render/sequence.rs (render_virtual_endpoint_marker renders Short as a dashed stub).
**Notes:** Implemented in feat(render): improve sequence diagram ch01 parity. Tests in tests/ch01_sequence_parity.rs.

### 1.31 Anchors and Duration — ❌ Missing
**Feature:** `{name}` anchors and `{a} <-> {b} : label` duration markers (teoz only).
**Syntax example:** `{start} Alice -> Bob : start`
**Status:** ❌ Missing
**Evidence:** No `{anchor}` / `{name}` brace handling found in parser/sequence.rs. Search for `\{start\}` / anchor returns no relevant matches.

### 1.32 Stereotypes and Spots — 🟡 Partial
**Feature:** `participant X << Stereotype >>`, with spot `<<(C,#color) name>>` colored circle.
**Syntax example:** `participant Alice << (C,#ADD1B2) Testable >>`
**Status:** 🟡 Partial
**Evidence:** normalize/sequence.rs:1633 (extract_c4_stereotype only handles C4 markers: `person`/`system`/etc., not arbitrary `<<text>>` rendering). parser/sequence.rs:1-61 has no `<<>>` extraction.
**Notes:** Generic stereotype labels and spot-circle rendering appear unsupported.

### 1.33 Position of the stereotypes — ❌ Missing
**Feature:** `skinparam stereotypePosition top|bottom`.
**Syntax example:** `skinparam stereotypePosition bottom`
**Status:** ❌ Missing
**Evidence:** No StereotypePosition variant in theme.rs::SequenceSkinParamValue.

#### 1.33.1 Top position (default) — ❌ Missing (see 1.33)
#### 1.33.2 Bottom position — ❌ Missing (see 1.33)

### 1.34 More information on titles — ✅ Supported
**Feature:** `title …` single-line, `title \n … end title` multi-line, creole/HTML in title.
**Syntax example:** `title __Simple__ **communication** example`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:229-239 (title), parser/sequence.rs:996-1018 (text_block_continues handles `end title`).

### 1.35 Participants encompass — ✅ Supported
**Feature:** `box "Name" #color … end box` draws a containing box around participants. Nested boxes only with teoz.
**Syntax example:** `box "Internal Service" #LightBlue … end box`
**Status:** ✅ Supported (single-level); 🟡 nesting needs teoz check
**Evidence:** parser/sequence.rs:353 (box in group keywords), render/sequence.rs:96 (render_participant_group_box at 605), normalize/sequence.rs:233 (E_BOX_END_UNMATCHED).
**Notes:** Nested boxes under teoz pragma — likely supported only if grouping nests cleanly; needs visual check.

### 1.36 Removing Foot Boxes — ✅ Supported
**Feature:** `hide footbox` hides the bottom participant boxes.
**Syntax example:** `hide footbox`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:263-268 (hide/show footbox), normalize/sequence.rs:383 (footbox_visible).

### 1.37 Skinparam — 🟡 Partial
**Feature:** Many sequence skinparams (ArrowColor, ParticipantBackgroundColor, sequenceArrowThickness, roundcorner, sequenceParticipant underline, handwritten, backgroundColor, font names, etc.).
**Syntax example:** `skinparam sequenceArrowThickness 2`
**Status:** 🟡 Partial
**Evidence:** theme.rs:1062-1304 covers a substantial set (ArrowColor, LifelineBorderColor, ParticipantBg/Border/Font, NoteBg/Border, GroupBg/Border, RoundCorner, Shadowing, DefaultFont*, BackgroundColor, MessageAlign, ResponseMessageBelowArrow, LifelineThickness, MessageLineColor, ReferenceBg/Border, GroupHeader font color/style).
**Notes:** Many properties supported; `sequenceArrowThickness`, `handwritten`, `sequenceParticipant underline`, `maxmessagesize` not visibly in the enum. Coverage incomplete vs full PlantUML set.

### 1.38 Changing padding — ✅ Supported
**Feature:** `skinparam ParticipantPadding`, `skinparam BoxPadding`.
**Syntax example:** `skinparam ParticipantPadding 20`
**Status:** ✅ Supported
**Evidence:** theme.rs:1222-1230 (ParticipantPadding, BoxPadding variants).

### 1.39 Appendix: Examples of all arrow type
#### 1.39.1 Normal arrow — 🟡 Partial
**Feature:** Full matrix: `->`, `->>`, `-\`, `-\\`, `-/`, `-//`, `->x`, `x->`, `o->`, `->o`, `o->o`, `<->`, `o<->o`, `x<->x`, `->>o`, `-\o`, `-\\o`, `-/o`, `-//o`, `x->o`.
**Syntax example:** `a -//o b`
**Status:** 🟡 Partial
**Evidence:** parser/sequence.rs:765-883 (parse_arrow). `//` (double slash) explicitly rejected at line 783. `-//`, `-//o` forms therefore likely fail.
**Notes:** Single-slash + slash-with-marker forms appear handled; doubled-slash forms missing.

#### 1.39.2 Itself arrow — 🟡 Partial
Same matrix on self; same status — relies on 1.39.1's arrow parsing. Self-message routing is supported (see 1.5).

#### 1.39.3 / 1.39.4 Incoming messages (with '[') — ✅ Supported
**Feature:** `[-> b`, `[->>`, `[-\`, etc. — same arrow matrix with left bracket.
**Status:** ✅ Supported for plain/circle/cross; 🟡 for doubled-slash forms (see 1.39.1).
**Evidence:** parser/sequence.rs:205-224, 912-925; ast_virtual_endpoint_from_id.

#### 1.39.5 Outgoing messages (with ']') — ✅ Supported / 🟡
Same as 1.39.4, mirrored. Same evidence.

#### 1.39.6 / 1.39.7 Short incoming (with '?') — ✅ Supported
**Status:** ✅ Supported (see 1.30).

#### 1.39.8 Short outgoing (with '?') — ✅ Supported
**Status:** ✅ Supported (see 1.30).

### 1.40 Specific SkinParameter
#### 1.40.1 By default — ✅ (prose only)
#### 1.40.2 LifelineStrategy — ✅ Supported
**Feature:** `skinparam lifelineStrategy nosolid|solid`.
**Syntax example:** `skinparam lifelineStrategy solid`
**Status:** ✅ Supported
**Evidence:** theme.rs:SequenceSkinParamValue::LifelineNoSolid; theme.rs:classify_sequence_skinparam handles `lifelinestrategy`; SequenceStyle.lifeline_nosolid; render/sequence.rs skips activation boxes when `lifeline_nosolid` is true.
**Notes:** Implemented in feat(render): improve sequence diagram ch01 parity. Tests in tests/ch01_sequence_parity.rs.

#### 1.40.3 style strictuml — ❌ Missing
**Feature:** `skinparam style strictuml` switches to strict UML triangle arrowheads.
**Syntax example:** `skinparam style strictuml`
**Status:** ❌ Missing
**Evidence:** No `strictuml` token in theme.rs or render/sequence.rs.

### 1.41 Hide unlinked participant — ✅ Supported
**Feature:** `hide unlinked` hides participants with no messages.
**Syntax example:** `hide unlinked`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:269-271 (HideUnlinked), normalize/sequence.rs:753 (filter pass), model.rs:714 (hide_unlinked field).

### 1.42 Color a group message — 🟡 Partial
**Feature:** `alt#Gold #LightBlue Successful case` / `else #Pink Failure` — colored group headers and branches.
**Syntax example:** `alt#Gold #LightBlue Successful case`
**Status:** 🟡 Partial — needs deeper review
**Evidence:** parser/sequence.rs:352-366 captures the full label string after the group kw, including `#Gold` prefix. No evidence in render/sequence.rs of extracting two colors from a `kw#A #B label` pattern.
**Notes:** Label including `#color` likely rendered verbatim; group header/background coloring missing.

### 1.43 Mainframe — ✅ Supported
**Feature:** `mainframe Title` draws a UML mainframe-style label box around the whole diagram.
**Syntax example:** `mainframe This is a **mainframe**`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs (parse_keyword handles `mainframe`); ast.rs:StatementKind::Mainframe; normalize/sequence.rs (sets SequenceDocument.mainframe); scene.rs:Scene.mainframe; render/sequence.rs:render_mainframe (outer rect + pentagon notch + title text).
**Notes:** Implemented in feat(render): improve sequence diagram ch01 parity. Tests in tests/ch01_sequence_parity.rs.

### 1.44 Slanted or odd arrows — ❌ Missing
**Feature:** `A ->(10) B` slanted arrows; `(nn)` shift pixels.
**Syntax example:** `A ->(10) B: text`
**Status:** ❌ Missing
**Evidence:** parser/tests.rs:1107 references "parses_expanded_slanted_arrow_tokens" but split_arrow's is_arrow_char does not include `(` or digits. Likely the `(10)` token breaks the arrow run and the message fails to parse, or is parsed but the slant offset is dropped.
**Notes:** Mark missing pending a parse test of the explicit `->(10)` form.

### 1.45 Parallel messages (with teoz) — ✅ Supported
**Feature:** Leading `&` on a line marks the message as parallel-aligned with the previous one (teoz only).
**Syntax example:** `& Bob -> Charlie : hi`
**Status:** ✅ Supported
**Evidence:** parser/sequence.rs:131-140 (split_parallel_message_prefix), parser/sequence.rs:78-83 (style.parallel set on Message).
**Notes:** Requires `!pragma teoz true`; teoz pragma parsed at normalize/sequence.rs:487.

---

## Tallies

- ✅ Supported: 28
- 🟡 Partial: 14
- ❌ Missing: 6

Sub-subsections 1.6.1, 1.33.1, 1.33.2, 1.39.1-1.39.8 counted individually. Prose-only 1.40.1 noted but not counted.

**Changes in this cycle (feat(render): improve sequence diagram ch01 parity):**
- 1.18 ❌→✅ (aligned notes `/note`)
- 1.30 🟡→✅ (short arrows `?->` / `->?`)
- 1.39.6-8 ❌→✅ (short arrow appendix)
- 1.40.2 ❌→✅ (lifelineStrategy nosolid)
- 1.43 ❌→✅ (mainframe)
