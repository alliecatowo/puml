# PicoUML Standard Library (stdlib)

Deterministic, text-stub compatibility shims for the most-used PlantUML icon and architecture libraries. Each file provides `!procedure` macros that produce labeled participants so diagrams parse, check, and render without requiring external network access or binary sprite data.

All files are resolved via angle-bracket includes (`!include <library/Module>`) when `--include-root` is set to the project root, or when the source file has a sibling `stdlib/` directory.

Run `puml -stdlib` to print the deterministic list of reachable local shim paths.
The output includes alias paths such as `awslib/... -> awslib14/...` and calls
out known upstream PlantUML stdlib packs that are not bundled here. This command
is an inventory/diagnostic surface only; PUML does not bulk-vendor or extract the
full third-party upstream packs.

---

## OpenIconic (`<&name>` / `&name`)

PUML also bundles the full Open Iconic SVG icon set as built-in sprite data. Use PlantUML/OpenIconic-style inline references directly in labels:

```plantuml
Alice -> Bob : Open <&folder,scale=2,color=#2563eb>
Bob --> Alice : Upload &cloud-upload complete
```

The icons are real inline SVG paths, do not require an `!include`, and appear in `listsprites` output alongside user-defined sprites. Open Iconic is MIT licensed; see `THIRD_PARTY_NOTICES.md` for attribution.

---

## Bootstrap Icons (`<$bi-name>`)

PUML bundles the full Bootstrap Icons SVG icon set as built-in `bi-` prefixed sprites, matching the PlantUML stdlib convention that avoids name collisions across icon families:

```plantuml
Alice -> Bob : Browse <$bi-globe,scale=2,color=#7952b3>
Bob --> Alice : Ship <$bi-box-seam{scale=2,color=#0d6efd}>
```

The icons are real inline SVG paths, do not require an `!include`, accept the normal sprite `scale` and `color` parameters, normalize `bi_foo` aliases to `bi-foo`, and appear in `listsprites` output alongside Open Iconic and user-defined sprites. Bootstrap Icons is MIT licensed; see `THIRD_PARTY_NOTICES.md` for attribution.

---

## C4 (`C4/`)

C4 architecture diagram library (C4-PlantUML).

| File | Key macros |
|------|-----------|
| `C4.puml` | `Person`, `Person_Ext`, `System`, `System_Ext`, `SystemDb`, `SystemQueue`, `Boundary`, `System_Boundary`, `Container_Boundary`, `Component_Boundary`, `Lay_R/L/U/D`, `Title`, `Footer`, `Legend`, `Rel`, `Rel_U/D/L/R`, `Rel_Back`, `Rel_Neighbor`, `BiRel`, `Rel_Back_Neighbor` |
| `C4_Context.puml` | Includes `C4` — context-view alias entry point |
| `C4_Container.puml` | `Container`, `ContainerDb`, `ContainerQueue`, `Container_Ext` |
| `C4_Component.puml` | `Component`, `ComponentDb`, `ComponentQueue`, `Component_Ext` |
| `C4_Deployment.puml` | `Deployment_Node`, `Deployment_Node_L`, `Deployment_Node_R`, `InfrastructureNode` |
| `C4_Dynamic.puml` | `Rel_Dynamic`, `Rel_Back`, `Rel_Neighbor`, `BiRel`, `Rel_Back_Neighbor` |
| `C4_Sequence.puml` | Sequence diagram alias for C4 |

---

## AWS Icons (`awslib14/`, reachable as `awslib/`)

Stub icon macros for Amazon Web Services resources (awslib14 layout).
The official PlantUML stdlib slug is `awslib`, and PUML maps
`!include <awslib/...>` / `!import awslib/...` to this bundled `awslib14/`
compatibility directory. The `awslib14` path is kept as a backwards-compatible
direct include form for existing fixtures.

### Compute (`awslib14/Compute/`)
`EC2`, `Lambda`, `ECS`, `EKS`, `Batch`, `Fargate`

### Storage (`awslib14/Storage/`)
`S3`, `EBS`, `EFS`, `Glacier`

### Database (`awslib14/Database/`)
`RDS`, `DynamoDB`, `Aurora`, `Redshift`, `ElastiCache`

### Networking (`awslib14/Networking/`)
`VPC`, `Route53`, `CloudFront`, `APIGateway`, `ELB` (also exports `ALB`, `NLB`)

### Security (`awslib14/Security/`)
`IAM`, `KMS`, `Cognito`, `WAF`

All AWS macros delegate to `AWSIcon($alias, ServiceName, $label, $descr)` from `AWSCommon.puml`.

---

## Azure (`azure/`)

Stub icon macros for Microsoft Azure services.

`AzureFunction`, `AzureSQLDatabase`, `AzureCosmosDB`, `AzureVM`, `AzureBlobStorage`, `AzureKubernetesService`, `AzureKeyVault`, `AzureActiveDirectory`, `AzureLogicApps`, `AzureServiceBus`

All macros delegate to `AzureIcon($alias, ServiceName, $label, $descr)` from `AzureCommon.puml`.

---

## GCP (`gcp/`)

Stub icon macros for Google Cloud Platform services.

`ComputeEngine`, `CloudStorage`, `BigQuery`, `CloudSQL`, `CloudFunctions`, `GKE`, `PubSub`, `CloudCDN`, `IAM`, `CloudLoadBalancing`

All macros delegate to `GCPIcon($alias, ServiceName, $label, $descr)` from `GCPCommon.puml`.

---

## Material Design (`material/`)

50+ common Material Design icon stubs. Each file exports one `MA_<NAME>($alias, $label, $descr)` macro.

`cloud`, `security`, `storage`, `database`, `smartphone`, `laptop`, `person`, `group`, `settings`, `network`, `router`, `server`, `email`, `message`, `notifications`, `lock`, `key`, `code`, `api`, `dashboard`, `analytics`, `search`, `home`, `build`, `share`, `queue`, `dns`, `vpn_lock`, `bug_report`, `cloud_upload`, `cloud_download`, `folder`, `file`, `schedule`, `account_circle`, `payment`, `public`, `devices`, `monitor`, `cached`, `sync`, `hub`, `token`, `policy`, `integration_instructions`, `terminal`, `data_object`, `table_chart`, `swap_horiz`, `manage_accounts`

---

## tupadr3 — Devicons (`tupadr3/devicons/`)

Developer technology icon stubs (25+ icons). Each exports `DEV_<NAME>($alias, $label, $descr)`.

`git`, `docker`, `kubernetes`, `python`, `java`, `javascript`, `typescript`, `rust`, `go`, `nodejs`, `react`, `graphql`, `postgresql`, `redis`, `mongodb`, `nginx`, `github`, `linux`, `apple`, `android`, `terraform`, `ansible`, `jenkins`, `kafka`, `elasticsearch`, `grafana`, `prometheus`

## tupadr3 — Font Awesome 5 (`tupadr3/font-awesome-5/`)

Font Awesome 5 icon stubs (28+ icons). Each exports `FA5_<NAME>($alias, $label, $descr)`.

`database`, `server`, `cloud`, `user`, `users`, `lock`, `shield_alt`, `envelope`, `mobile_alt`, `laptop`, `network_wired`, `globe`, `key`, `code`, `cogs`, `chart_bar`, `file_alt`, `shopping_cart`, `search`, `bell`, `home`, `tasks`, `sync`, `credit_card`, `map_marker`, `calendar`, `plug`, `microchip`, `stream`, `sitemap`

---

## Office (`office/`)

Microsoft Office icon stubs organized by category.

### Servers (`office/Servers/`)
`web_server`, `application_server`, `database_server`, `file_server`, `mail_server`

### Devices (`office/Devices/`)
`laptop`, `smartphone`, `desktop`, `tablet`, `printer`

All macros delegate to `OfficeIcon($alias, Category_name, $label, $descr)` from `office/common.puml`.
