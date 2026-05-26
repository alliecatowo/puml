+++
title = "Themes and styling"
description = "Skinparams, themes, and visual token overrides."
weight = 90
+++

`puml` treats themes as **token overrides passed into the renderer** &mdash; never as CSS hacks layered over emitted SVG. The renderer reads a token bag at compile time and bakes the values directly into the output.

## Built-in themes

```puml
!theme spacelab
```

The [`themes/` gallery](@/gallery.md) shows the built-in palettes against every core family. Unknown theme names emit a deterministic warning &mdash; they're not parser errors.

## Skinparams

```puml
skinparam backgroundColor #fafafa
skinparam defaultFontName "Inter"
skinparam shadowing false

skinparam sequence {
  ParticipantBorderColor    #333
  ArrowColor                #111
  ArrowFontColor            #555
  LifeLineBorderColor       #888
  LifeLineBackgroundColor   #eee
  GroupBorderColor          #888
  GroupFontColor            #444
}

skinparam class {
  BackgroundColor    #ffffff
  BorderColor        #444
  ArrowColor         #444
  StereotypeFontColor #6a5acd
}
```

> Unsupported skinparam keys emit non-fatal warnings on stderr. The deterministic catalog of known-but-unsupported keys lives in the syntax-highlighting spec.

## Per-arrow styling

You can color and style individual arrows inline:

```puml
Alice -[#blue,dashed]-> Bob: queued
Alice -[#FF8800,bold]-> Bob: urgent
```

Color values accept `#hex`, named colors, and the named palette from PlantUML.

## Per-element styling

For class diagrams, stereotypes can carry inline color:

```puml
class Service <<(S,#7dd3fc) Service>>
class Repository <<(R,#34d399) Data>>
```

## Determinism

Themes and skinparams **do not** affect determinism. Two runs with identical source and identical effective tokens produce byte-identical SVG.

## See the gallery

- [`themes/`](@/gallery.md) &mdash; built-in theme variants across families.
- [`skinparams/`](@/gallery.md) &mdash; explicit skinparam coverage.
- [`creole/`](@/gallery.md) &mdash; rich-text in labels and notes.
