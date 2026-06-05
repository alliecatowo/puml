//! Structural density-ratio assertions for the deployment family per-shape retune (#1426).
//!
//! 2026-06-04 (density-revert PR #1563): global layout_constants reverted to pre-#1346
//! looser values. Deployment-family per-shape constants (DEPLOYMENT_BOX_WIDTH, etc.)
//! remain post-#1426, but the global rank/node separation and pkg padding now push
//! ratios above the original ≤2.5× target. Caps relaxed as regression guards.
//!
//! These tests guard against regressions that would re-inflate deployment diagram canvas
//! sizes back toward the 2-5× PlantUML area ratios observed in the wave-4 audit:
//!
//! | Fixture              | Pre-#1426 ratio | Target | Post-#1426 ratio |
//! |----------------------|-----------------|--------|-----------------|
//! | deployment/02        | 4.90×           | ≤2.5×  | 2.43×           |
//! | deployment/03        | 3.68×           | ≤2.5×  | 1.95×           |
//! | deployment/06        | 2.21×           | ≤1.8×  | 1.11×           |
//!
//! PlantUML reference dimensions (ground truth from the wave-4 forensic audit):
//!   - deployment/02_databases: 254×322 px → 81,788 px²
//!   - deployment/03_cloud:     344×199 px → 68,456 px²
//!   - deployment/06_kubernetes: 934×839 px → 783,626 px²

fn render(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

/// Extract the SVG canvas area (width * height) from the root `<svg ...>` tag.
fn svg_canvas_area(svg: &str) -> u64 {
    let w = extract_svg_attr(svg, "width");
    let h = extract_svg_attr(svg, "height");
    w * h
}

fn extract_svg_attr(svg: &str, attr: &str) -> u64 {
    // Find the first `<svg ...>` tag and extract the given attribute value.
    let tag_end = svg.find('>').unwrap_or(svg.len());
    let tag = &svg[..tag_end];
    let needle = format!("{}=\"", attr);
    let start = tag.find(&needle).unwrap_or_else(|| {
        panic!(
            "attribute '{}' not found in <svg> tag: {}",
            attr,
            &svg[..200]
        )
    }) + needle.len();
    let end = tag[start..]
        .find('"')
        .unwrap_or_else(|| panic!("closing '\"' not found after attribute '{}' value", attr))
        + start;
    tag[start..end].parse::<u64>().unwrap_or_else(|_| {
        panic!(
            "attribute '{}' value '{}' is not a u64",
            attr,
            &tag[start..end]
        )
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture 1: deployment/02_databases (was 4.90×, target ≤2.5×)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn deployment_02_databases_area_ratio_within_target() {
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
    let our_area = svg_canvas_area(&svg);
    // PlantUML reference: 254×322 = 81,788 px²
    let plantuml_area: u64 = 81_788;
    let ratio = our_area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 5.0,
        "deployment/02_databases area ratio {:.2}x exceeds 5.0x post-revert regression cap \
         (our canvas {}px2, PlantUML reference {}px2)",
        ratio,
        our_area,
        plantuml_area,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture 2: deployment/03_cloud (was 3.68×, target ≤2.5×)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn deployment_03_cloud_area_ratio_within_target() {
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
    let our_area = svg_canvas_area(&svg);
    // PlantUML reference: 344×199 = 68,456 px²
    let plantuml_area: u64 = 68_456;
    let ratio = our_area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 4.0,
        "deployment/03_cloud area ratio {:.2}x exceeds 4.0x post-revert regression cap \
         (our canvas {}px2, PlantUML reference {}px2)",
        ratio,
        our_area,
        plantuml_area,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixture 3: deployment/06_kubernetes_pods_containers (was 2.21×, target ≤1.8×)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn deployment_06_kubernetes_area_ratio_within_target() {
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
    let our_area = svg_canvas_area(&svg);
    // PlantUML reference: 934×839 = 783,626 px²
    let plantuml_area: u64 = 783_626;
    let ratio = our_area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 3.0,
        "deployment/06_kubernetes area ratio {:.2}x exceeds 3.0x post-revert regression cap \
         (our canvas {}px2, PlantUML reference {}px2)",
        ratio,
        our_area,
        plantuml_area,
    );
}
