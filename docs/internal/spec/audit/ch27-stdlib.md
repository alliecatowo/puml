# Chapter 27 — PlantUML Standard Library: PUML Renderer Audit

Status legend: ✅ implemented · 🟡 partial · ❌ not implemented

---

## Bundled Stdlib Directory

`ls /Users/allison.coleman/Develop/puml/stdlib/` returns:

```
awslib14/  azure/  C4/  gcp/  material/  office/  tupadr3/  README.md
```

Resolution mechanism: angle-bracket form `!include <Lib/Module>` is recognised in `src/preproc/includes.rs:139,468` (`process_stdlib_angle_include`) and resolves against `PUML_STDLIB_ROOT` / `--include-root` / a sibling `stdlib/` directory (`includes.rs:494-505`). All angle-bracket includes are forced include-once (`includes.rs:507`).

**Important caveat:** The bundled `*.puml` files are **simplified compatibility shims**, not verbatim copies of the upstream plantuml-stdlib. For example, `stdlib/awslib14/AWSCommon.puml` contains macro stubs like `!procedure AWSIcon($alias, $service, $label="", $descr="")\n  object $label as $alias <<aws-$service>>\n!endprocedure` — the underlying sprite is **not** defined (see ch23: sprite definition is a no-op), so AWS icons render as plain `<<aws-foo>>` stereotyped objects.

---

### 27.1 `stdlib` listing diagram (`@startuml\nstdlib\n@enduml`) — ❌
**Feature:** Special diagram type that lists every bundled stdlib folder.
**Syntax example:** `stdlib`
**Status:** ❌
**Evidence:** No `stdlib` keyword in parser. Grep returns no diagram-level handler.

### 27.1 `-stdlib` / `-extractstdlib` CLI — ❌
**Feature:** CLI listing + extraction commands.
**Status:** ❌
**Evidence:** No flags in `src/cli.rs`.

### 27.2 ArchiMate [archimate] — ❌
**Feature:** `!include <archimate/Archimate>` macros (`Business_Object(...)`, `Rel_Flow_Left(...)`, sprites).
**Status:** ❌ — directory **not bundled**. `ls stdlib/archimate` → does not exist. `!include <archimate/...>` resolves to a missing-file error.

### 27.3 Amazon Labs AWS Library [awslib] — 🟡
**Feature:** `!include <awslib/AWSCommon>` + per-service icon includes (`!include <awslib/Analytics/KinesisDataStreams>`).
**Status:** 🟡 — folder name is **`awslib14`** (not `awslib`). Provides AWSCommon.puml + Compute, Database, Networking, Security, Storage subfolders. Macros expand to stereotyped `object` calls but sprite icons are inactive (ch23).
**Evidence:** `stdlib/awslib14/AWSCommon.puml` (macro shims only).
**Notes:** Spec uses `!include <awslib/...>` — that exact form will fail; users must write `!include <awslib14/...>`.

### 27.4 Azure library [azure] — 🟡
**Feature:** `!include <azure/AzureCommon>` + per-service. Macros + sprites.
**Status:** 🟡 — bundled at `stdlib/azure/`. Files include AzureActiveDirectory, AzureBlobStorage, AzureCommon, AzureCosmosDB, AzureFunction, AzureKeyVault, AzureKubernetesService, AzureLogicApps, AzureServiceBus, AzureSQLDatabase. Macros work; sprites don't.

### 27.5 C4 Library [C4] — 🟡
**Feature:** `!include <C4/C4_Container>` etc.; `Person()`, `Container()`, `System()`, `Rel()`, `Rel_U()` macros.
**Status:** 🟡 — bundled: C4.puml, C4_Component.puml, C4_Container.puml, C4_Context.puml, C4_Deployment.puml, C4_Dynamic.puml, C4_Sequence.puml. Macros expand to component/relationship calls. C4 is the most viable bundled library since it's macro-based and doesn't depend on sprites.

### 27.6 Cloud Insight [cloudinsight] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.7 Cloudogu [cloudogu] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.8 EDGY [edgy] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.9 Elastic library [elastic] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.10 Google Material Icons [material] — 🟡
**Status:** 🟡 — bundled at `stdlib/material/` with ~hundreds of `*.puml` files (e.g. `account_circle.puml`, `analytics.puml`, `api.puml`, `bug_report.puml`, ...). Same caveat: sprite-only — sprites are no-ops, so includes succeed but icons don't render.

### 27.11 Kubernetes [kubernetes] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.12 Logos [logos] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.13 Office [office] — 🟡
**Status:** 🟡 — bundled at `stdlib/office/` with `common.puml`, `Office.puml`, plus `Devices/` and `Servers/` subdirs.

### 27.14 Open Security Architecture (OSA) [osa] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.15 Tupadr3 library [tupadr3] — 🟡
**Status:** 🟡 — bundled at `stdlib/tupadr3/` with `common.puml`, `devicons/` (~hundreds of icons: android, ansible, apple, docker, elasticsearch, ...), and `font-awesome-5/`.

### 27.16 GCP library [gcp] — 🟡
**Status:** 🟡 — bundled at `stdlib/gcp/`. Files: BigQuery, CloudCDN, CloudFunctions, CloudLoadBalancing, CloudSQL, CloudStorage, ComputeEngine, GCPCommon, GKE, IAM.
**Notes:** Spec uses `[aws]` at §27.16 but the project ships GCP under that slot — these are different libraries; coverage of the **second AWS variant** (`!include <aws/AWSCommon>`) is ❌ (only `awslib14` is bundled).

### 27.17 listsprite / listsprites (per-library) — ❌
**Status:** ❌ — see ch23.

---

## Tally — Bundled vs Specified Libraries

| Library [tag] | Bundled? | Macros work | Icons (sprites) work |
|---|---|---|---|
| archimate | ❌ | — | — |
| awslib (as `awslib14`) | 🟡 (wrong slug) | ✅ | ❌ |
| azure | ✅ | ✅ | ❌ |
| C4 | ✅ | ✅ | N/A (macro-only) |
| cloudinsight | ❌ | — | — |
| cloudogu | ❌ | — | — |
| edgy | ❌ | — | — |
| elastic | ❌ | — | — |
| material | ✅ | ✅ | ❌ |
| kubernetes | ❌ | — | — |
| logos | ❌ | — | — |
| office | ✅ | ✅ | ❌ |
| osa | ❌ | — | — |
| tupadr3 (devicons + fa5) | ✅ | ✅ | ❌ |
| gcp | ✅ | ✅ | ❌ |
| aws (second variant §27.16) | ❌ | — | — |

**Mechanics:** `stdlib` diagram ❌ · `-stdlib` CLI ❌ · `-extractstdlib` CLI ❌ · `listsprites` ❌ · angle-bracket resolver ✅ · `%get_all_stdlib` ❌ (stub).

**Score:** 7 of 16 listed libraries are at least partially bundled (44%). Because **sprites are inert** (ch23), every icon library is effectively macro-shim-only — diagrams compile but render stereotyped boxes instead of icons. **C4 is the only library that delivers its full intended UX** because it does not depend on sprite rendering.
