# Chapter 27 — PlantUML Standard Library: PUML Renderer Audit

Status legend: ✅ implemented · 🟡 partial · ❌ not implemented

---

## Bundled Stdlib Directory

`find stdlib -mindepth 1 -maxdepth 1 -type d | sort` returns:

```
C4/  awslib14/  azure/  gcp/  material/  office/  tupadr3/
```

Resolution mechanism: angle-bracket form `!include <Lib/Module>` is recognised in `src/preproc/includes.rs` (`process_stdlib_angle_include`) and resolves against `PUML_STDLIB_ROOT` / `--include-root` / a sibling `stdlib/` directory / the dev/test `CARGO_MANIFEST_DIR/stdlib` fallback. All angle-bracket includes are forced include-once. Import form `!import Lib/Module` resolves through the stdlib root as well. The shared inventory helper in `src/stdlib.rs` scans the same local `stdlib/` tree for deterministic listing surfaces.

Slug aliases: `awslib/...` maps to the bundled `awslib14/...` compatibility directory for `!include <awslib/...>`, `!import awslib/...`, `-stdlib`, and `%get_all_stdlib()` (`src/preproc/includes.rs`, `src/stdlib.rs`, `tests/fixtures/include/valid_awslib_ec2.puml`, `src/parser/tests.rs`, `tests/cli_stdlib.rs`, `tests/coverage_wave23_builtins.rs`). The direct `awslib14/...` path remains supported for existing fixtures.

**Important caveat:** The bundled `*.puml` files are **simplified compatibility shims**, not verbatim copies of the upstream plantuml-stdlib. Sprite parsing/rendering now works (see ch23), and several local icon shims define small deterministic sprites. Bootstrap Icons is an exception: it is bundled as generated built-in SVG sprite data rather than a `stdlib/bootstrap/` directory. AWS/Azure/GCP/Office compatibility files still primarily expand to labelled/stereotyped objects rather than full upstream icon art.

**Upstream comparison source:** The official `plantuml/plantuml-stdlib` README currently lists AdaML `[ada]`, Amazon Web Services `[aws]`, Amazon Labs AWS `[awslib]`, Azure `[azure]`, Bootstrap `[bootstrap]`, C4 `[C4]`, Classy `[classy]`, Classy C4 `[classy-c4]`, DomainStory `[DomainStory]`, Edgy `[edgy]`, EIP `[eip]`, Elastic `[elastic]`, GCP `[gcp]`, K8S `[k8s]`, Material `[material, material2, material7]`, Tupadr3 `[tupadr3]`, plus an ArchiMate section below the summary list.

---

### 27.1 `stdlib` listing diagram (`@startuml\nstdlib\n@enduml`) — ❌
**Feature:** Special diagram type that lists every bundled stdlib folder.
**Syntax example:** `stdlib`
**Status:** ❌
**Evidence:** No `stdlib` keyword in parser. Grep returns no diagram-level handler.

### 27.1 `-stdlib` / `-extractstdlib` CLI — 🟡
**Feature:** CLI listing + extraction commands.
**Status:** 🟡 — `puml -stdlib` and `puml --stdlib` list the reachable local shim include paths, including alias entries such as `awslib/Compute/EC2.puml -> awslib14/Compute/EC2.puml`. Output is sorted and starts with comments documenting the local root, alias mapping, and known missing upstream packs. `-extractstdlib` remains intentionally unsupported; PUML does not bulk-vendor or extract full upstream third-party packs.
**Evidence:** `Cli::stdlib` (`src/cli.rs`), PlantUML single-dash expansion (`src/main.rs`), formatter/inventory helper (`src/stdlib.rs`), CLI coverage (`tests/cli_stdlib.rs`).

### 27.2 ArchiMate [archimate] — ❌
**Feature:** `!include <archimate/Archimate>` macros (`Business_Object(...)`, `Rel_Flow_Left(...)`, sprites).
**Status:** ❌ — directory **not bundled**. `ls stdlib/archimate` → does not exist. `!include <archimate/...>` resolves to a missing-file error.

### 27.3 Amazon Labs AWS Library [awslib] — 🟡
**Feature:** `!include <awslib/AWSCommon>` + per-service icon includes (`!include <awslib/Analytics/KinesisDataStreams>`).
**Status:** 🟡 — reachable through the official `awslib` slug via a resolver alias to the bundled `awslib14` directory. Provides AWSCommon.puml + Compute, Database, Networking, Security, Storage subfolders. Macros expand to stereotyped `object` calls; the bundled surface is still a small shim subset rather than the full upstream awslib pack.
**Evidence:** `stdlib/awslib14/AWSCommon.puml`; `tests/fixtures/include/valid_awslib_ec2.puml` uses `!include <awslib/Compute/EC2>` and `!include <awslib/AWSCommon>`; `src/parser/tests.rs` covers both `!include <awslib/...>` and `!import awslib/...`.
**Notes:** Direct `!include <awslib14/...>` remains accepted for backwards compatibility, but docs and new tests should prefer the PlantUML slug `awslib`.

### 27.4 Azure library [azure] — 🟡
**Feature:** `!include <azure/AzureCommon>` + per-service. Macros + sprites.
**Status:** 🟡 — bundled at `stdlib/azure/`. Files include AzureActiveDirectory, AzureBlobStorage, AzureCommon, AzureCosmosDB, AzureFunction, AzureKeyVault, AzureKubernetesService, AzureLogicApps, AzureServiceBus, AzureSQLDatabase. Macros work; sprites don't.

### 27.5 C4 Library [C4] — 🟡
**Feature:** `!include <C4/C4_Container>` etc.; `Person()`, `Container()`, `System()`, `Rel()`, `Rel_U()` macros.
**Status:** 🟡 — bundled: C4.puml, C4_Component.puml, C4_Container.puml, C4_Context.puml, C4_Deployment.puml, C4_Dynamic.puml, C4_Sequence.puml. Macros expand to component/relationship calls. C4 is the most viable bundled library since it's macro-based and doesn't depend on sprites.

### 27.5a Bootstrap Icons [bootstrap] — 🟡
**Feature:** `!include <bootstrap/bootstrap>` plus `bi-` prefixed SVG sprites such as `<$bi-globe>`.
**Status:** 🟡 — full Bootstrap Icons 1.13.1 SVG sprite art is bundled as generated built-in data and available through `<$bi-name>` / `<$bi_name>` references without an include. The `stdlib/bootstrap/` include entry point and helper macros remain unimplemented.
**Evidence:** `src/bootstrap_icons.rs` contains 2,078 generated SVG entries; `src/sprites.rs` resolves the PlantUML stdlib `bi-` prefix; `src/render/svg.rs` includes Bootstrap Icons in `listsprites`; `tests/integration.rs` covers inline rendering and list metadata.
**Notes:** Attribution is recorded in `THIRD_PARTY_NOTICES.md`.

### 27.6 Cloud Insight [cloudinsight] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.7 Cloudogu [cloudogu] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.8 EDGY [edgy] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.9 Elastic library [elastic] — ❌
**Status:** ❌ — directory **not bundled**.

### 27.10 Google Material Icons [material] — 🟡
**Status:** 🟡 — bundled at `stdlib/material/` with 52 deterministic shim files (e.g. `account_circle.puml`, `analytics.puml`, `api.puml`, `bug_report.puml`, ...). Sprite includes render through the ch23 sprite path, but the local pack is a small curated subset and does not provide the official `material2` or `material7` slugs.

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
**Notes:** Coverage of the separate upstream AWS variant (`!include <aws/...>`) is still ❌; only the awslib compatibility subset is bundled.

### 27.17 listsprite / listsprites (per-library) — ❌
**Status:** ✅ — see ch23 for `listsprite` / `listsprites` parsing and sprite sheet rendering. Chapter 27 still lacks the stdlib inventory diagram and CLI listing commands.

---

## Tally — Bundled vs Specified Libraries

| Library [tag] | Bundled? | Macros work | Icons (sprites) work |
|---|---|---|---|
| archimate | ❌ | — | — |
| awslib (aliased to `awslib14`) | 🟡 | ✅ | 🟡 |
| azure | ✅ | ✅ | 🟡 |
| bootstrap | 🟡 | ❌ | ✅ |
| C4 | ✅ | ✅ | N/A (macro-only) |
| cloudinsight | ❌ | — | — |
| cloudogu | ❌ | — | — |
| edgy | ❌ | — | — |
| elastic | ❌ | — | — |
| material | ✅ | ✅ | ✅ |
| kubernetes | ❌ | — | — |
| logos | ❌ | — | — |
| office | ✅ | ✅ | 🟡 |
| osa | ❌ | — | — |
| tupadr3 (devicons + fa5) | ✅ | ✅ | ✅ |
| gcp | ✅ | ✅ | 🟡 |
| aws (second variant §27.16) | ❌ | — | — |

**Mechanics:** `stdlib` diagram ❌ · `-stdlib` CLI 🟡 local shim listing · `-extractstdlib` CLI ❌ · `listsprites` ✅ (see ch23) · angle-bracket resolver ✅ · `%get_all_stdlib` 🟡 local shim path list.

**Score:** 8 of the locally tracked 17 library rows are at least partially bundled (47%). Against the current upstream README summary, PUML has partial coverage for `awslib`, `azure`, Bootstrap Icons, `C4`, `gcp`, `material`, and `tupadr3`, plus local-only `office`; it lacks AdaML, `aws`, Classy, Classy C4, DomainStory, Edgy, EIP, Elastic, K8S, `material2`, `material7`, and ArchiMate as bundled stdlib directories. C4 remains the closest to full intended UX because it is mostly macro-based; most icon-library directory shims are curated deterministic subsets, while Bootstrap Icons is bundled as generated SVG sprite data.
