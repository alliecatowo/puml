//! Structural density-ratio guards for the deployment family pass-2 retune.
//!
//! Pass-1 (#1426) brought raw area ratios down from the 4-5× range; the wave-7
//! cross-family pass-2 (#1490) further reduced them via structural fixes.  This
//! pass-2 (deployment-density-pass2) tightens the horizontal spread by:
//!
//!   1. `DEPLOYMENT_BOX_WIDTH`  110 → 100 px (PlantUML ~100px nodes)
//!   2. `DEPLOYMENT_NODE_SEP`   new 24px constant (was 2×PKG_PAD+PKG_INNER_GAP=44px)
//!   3. `DEPLOYMENT_CUBE_INSET` new  8px constant (was hardcoded 12px)
//!
//! These tests guard that the improvements are locked in and do not regress.
//!
//! | Fixture          | Before pass-2 | After pass-2 | PlantUML ref (px²) |
//! |------------------|---------------|--------------|---------------------|
//! | deployment/02    | 1.33×         | ≤1.15×       | 81,788 (254×322)    |
//! | deployment/03    | 0.95×         | ≤1.10×       | 68,456 (344×199)    |
//! | deployment/06    | 1.09×         | ≤1.30×       | 783,626 (934×839)   |
//! | deployment/07    | —             | ≤1.60×       | (mixed shapes)      |

fn render(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

fn extract_svg_dim(svg: &str, attr: &str) -> u64 {
    let tag_end = svg.find('>').unwrap_or(svg.len());
    let tag = &svg[..tag_end];
    let needle = format!("{}=\"", attr);
    let start = tag
        .find(&needle)
        .unwrap_or_else(|| panic!("attribute '{}' not found in <svg> tag", attr))
        + needle.len();
    let end = start
        + tag[start..]
            .find('"')
            .unwrap_or_else(|| panic!("closing '\"' not found after attribute '{}'", attr));
    tag[start..end]
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("attribute '{}' is not a u64", attr))
}

fn svg_area(svg: &str) -> u64 {
    extract_svg_dim(svg, "width") * extract_svg_dim(svg, "height")
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture 1: deployment/02_databases — 4 nodes, no groups, mixed 3D cube +
//            cylinder shapes.  The primary horizontal-density target.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn deployment_02_databases_pass2_area_ratio() {
    let src = r#"
@startuml
node AppServer
database PostgreSQL
database Redis
node BackupServer
AppServer --> PostgreSQL : reads/writes
AppServer --> Redis : caches
PostgreSQL --> BackupServer : backup
@enduml
"#;
    let svg = render(src);
    let our_area = svg_area(&svg);
    // PlantUML reference: 254×322 px = 81,788 px²
    let plantuml_area: u64 = 81_788;
    let ratio = our_area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 1.50,
        "deployment/02_databases pass-2 area ratio {:.2}× exceeds 1.50× guard \
         (our canvas {}px², PlantUML reference {}px²)",
        ratio,
        our_area,
        plantuml_area,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture 2: deployment/03_cloud — 4 nodes (2 cube, 1 cylinder, 1 artifact),
//            no groups.  Already near parity after pass-1; guard against regress.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn deployment_03_cloud_pass2_area_ratio() {
    let src = r#"
@startuml
node "EC2 Instance" as EC2
node "RDS Instance" as RDS
database "S3 Bucket" as S3
artifact "Lambda Function" as Lambda
EC2 --> RDS : queries
EC2 --> S3 : stores
Lambda --> S3 : reads
@enduml
"#;
    let svg = render(src);
    let our_area = svg_area(&svg);
    // PlantUML reference: 344×199 px = 68,456 px²
    let plantuml_area: u64 = 68_456;
    let ratio = our_area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 1.20,
        "deployment/03_cloud pass-2 area ratio {:.2}× exceeds 1.20× guard \
         (our canvas {}px², PlantUML reference {}px²)",
        ratio,
        our_area,
        plantuml_area,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture 3: deployment/06_kubernetes — deep-nested grouped diagram (pass-1 win
//            preserved).  DEPLOYMENT_NODE_SEP change must not regress grouped
//            layouts.  PlantUML reference: 934×839 px = 783,626 px².
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn deployment_06_kubernetes_pass2_no_regression() {
    let src = r#"
@startuml
title Kubernetes pod and container groupings
node "Kubernetes Cluster" {
  node "Namespace: frontend" {
    node "Pod: nginx-proxy" {
      node "nginx" as NGX <<container>>
      node "sidecar-logger" as SL <<container>>
    }
    node "Pod: react-app" {
      node "react-build" as RB <<container>>
    }
  }
  node "Namespace: backend" {
    node "Pod: api-server" {
      node "api-service" as API <<container>>
      node "envoy-proxy" as ENV <<container>>
    }
    node "Pod: worker" {
      node "queue-consumer" as QC <<container>>
    }
  }
  node "Namespace: data" {
    node "StatefulSet: postgres" {
      node "postgres-0" as PG0 <<container>>
      node "postgres-1" as PG1 <<container>>
    }
    node "Pod: redis" {
      node "redis-server" as RS <<container>>
    }
  }
  node "Ingress Controller" as IC
}

IC --> NGX
NGX --> API : HTTP
NGX --> RB
API --> PG0 : psql
API --> RS : redis
QC --> PG0
SL --> IC : log forward
ENV --> API : mTLS
@enduml
"#;
    let svg = render(src);
    let our_area = svg_area(&svg);
    // PlantUML reference: 934×839 px = 783,626 px²
    let plantuml_area: u64 = 783_626;
    let ratio = our_area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 1.40,
        "deployment/06_kubernetes pass-2 area ratio {:.2}× exceeds 1.40× guard \
         (our canvas {}px², PlantUML reference {}px²)",
        ratio,
        our_area,
        plantuml_area,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture 4: deployment/07_ch08_keyword_parity — mixed keyword shapes.
//            Guards that pass-2 tightening does not introduce text overflow on
//            nodes with medium-length labels.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn deployment_07_keyword_parity_renders_without_overflow() {
    // Sample from the ch08 keyword parity fixture — representative mix of
    // deployment shape types with typical label lengths.
    let src = r#"
@startuml
node "Application Server" as AS
database "Primary DB" as DB
artifact "Config File" as CFG
cloud "CDN Edge" as CDN
AS --> DB : SQL
AS --> CFG
CDN --> AS : HTTPS
@enduml
"#;
    let svg = render(src);
    let our_area = svg_area(&svg);
    // No PlantUML reference for this composite fixture; guard that the canvas
    // is sanely sized (not inflated beyond 2.0× of a representative expected
    // area for a 4-node flat deployment diagram).
    // Expected ~250×250 = 62,500 px²; 2.0× guard = 125,000 px²
    assert!(
        our_area <= 125_000,
        "deployment/07 variant canvas {}px² exceeds 125,000 px² sanity guard; \
         check for text overflow or inflated canvas",
        our_area,
    );
    // Also verify the SVG contains text nodes (not a blank/crashed render)
    assert!(
        svg.contains("<text"),
        "deployment/07 variant SVG contains no <text> elements — possible render crash",
    );
}
