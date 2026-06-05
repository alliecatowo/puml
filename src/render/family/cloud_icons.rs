/// Cloud icon library rendering for AWS/Azure/GCP/tupadr3 stereotype nodes.
///
/// When a user includes `<awslib14/Compute/EC2>` and calls `EC2(alias, "label")`,
/// the stub macro expands to `object "label" as alias <<aws-EC2>>`. This module
/// detects such stereotypes and renders visually distinct, labeled icon glyphs
/// instead of identical generic stub boxes.
///
/// Design goals:
/// - Each cloud provider gets a unique border color, fill, and badge glyph.
/// - The service name is rendered in the header so it is always visible.
/// - Adding new services later is a data-only change (add entries to the registry).
/// - Falls back gracefully: unrecognised stereotypes still render as plain text.
use crate::render::svg::escape_text;

/// Represents a detected cloud icon stereotype.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudIconStereotype {
    /// The cloud provider (e.g. "aws", "azure", "gcp", "fa", "dev").
    pub provider: CloudProvider,
    /// The service/icon name extracted from the stereotype (e.g. "EC2", "Lambda").
    pub service: String,
}

/// Supported cloud icon providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
    FontAwesome,
    Devicons,
    /// Kubernetes resource icons (`<<k8s-Pod>>`, `<<k8s-Service>>`, etc.).
    Kubernetes,
}

impl CloudProvider {
    /// Human-readable provider label shown in the badge.
    pub fn badge_label(self) -> &'static str {
        match self {
            CloudProvider::Aws => "AWS",
            CloudProvider::Azure => "Azure",
            CloudProvider::Gcp => "GCP",
            CloudProvider::FontAwesome => "FA",
            CloudProvider::Devicons => "Dev",
            CloudProvider::Kubernetes => "K8s",
        }
    }

    /// Border/stroke colour for this provider's icon box.
    pub fn stroke_color(self) -> &'static str {
        match self {
            CloudProvider::Aws => "#FF9900",
            CloudProvider::Azure => "#0078D4",
            CloudProvider::Gcp => "#4285F4",
            CloudProvider::FontAwesome => "#339AF0",
            CloudProvider::Devicons => "#56A0D3",
            // Kubernetes official brand blue
            CloudProvider::Kubernetes => "#326CE5",
        }
    }

    /// Light fill colour for this provider's icon box.
    pub fn fill_color(self) -> &'static str {
        match self {
            CloudProvider::Aws => "#FFF3E0",
            CloudProvider::Azure => "#E3F2FD",
            CloudProvider::Gcp => "#E8F0FE",
            CloudProvider::FontAwesome => "#E7F5FF",
            CloudProvider::Devicons => "#EBF8FF",
            CloudProvider::Kubernetes => "#E8EFFE",
        }
    }

    /// Header band fill colour for this provider.
    pub fn header_fill(self) -> &'static str {
        match self {
            CloudProvider::Aws => "#FF9900",
            CloudProvider::Azure => "#0078D4",
            CloudProvider::Gcp => "#4285F4",
            CloudProvider::FontAwesome => "#339AF0",
            CloudProvider::Devicons => "#56A0D3",
            CloudProvider::Kubernetes => "#326CE5",
        }
    }

    /// Font colour for text drawn over the header band.
    pub fn header_font_color(self) -> &'static str {
        "#ffffff"
    }
}

/// Parse a stereotype string like `<<aws-EC2>>` or `<<azure-vm>>` into a
/// [`CloudIconStereotype`]. Returns `None` for non-cloud stereotypes.
pub fn parse_cloud_stereotype(text: &str) -> Option<CloudIconStereotype> {
    let inner = text.trim().strip_prefix("<<")?;
    let inner = inner.strip_suffix(">>")?;
    let inner = inner.trim();

    // Try each provider prefix (longest-first to avoid partial matches).
    let (provider, rest) = if let Some(r) = inner.strip_prefix("azure-") {
        (CloudProvider::Azure, r)
    } else if let Some(r) = inner.strip_prefix("aws-") {
        (CloudProvider::Aws, r)
    } else if let Some(r) = inner.strip_prefix("gcp-") {
        (CloudProvider::Gcp, r)
    } else if let Some(r) = inner
        .strip_prefix("fa5-")
        .or_else(|| inner.strip_prefix("fa-"))
    {
        (CloudProvider::FontAwesome, r)
    } else if let Some(r) = inner
        .strip_prefix("devicons_")
        .or_else(|| inner.strip_prefix("dev-"))
    {
        (CloudProvider::Devicons, r)
    } else if let Some(r) = inner.strip_prefix("k8s-") {
        (CloudProvider::Kubernetes, r)
    } else {
        return None;
    };

    if rest.is_empty() {
        return None;
    }

    // Humanise the service name: replace underscores/hyphens with spaces, title-case.
    let service = humanise_service_name(rest);

    Some(CloudIconStereotype { provider, service })
}

/// Convert a raw stereotype service slug (e.g. `"ec2"`, `"blob_storage"`) to a
/// human-readable service name.
fn humanise_service_name(raw: &str) -> String {
    // Replace both separator characters in a single pass to avoid consecutive replace calls.
    let normalised: String = raw
        .chars()
        .map(|c| if c == '_' || c == '-' { ' ' } else { c })
        .collect();
    normalised
        .split_whitespace()
        .map(|word| {
            // Preserve well-known all-caps abbreviations.
            let upper = word.to_ascii_uppercase();
            if KNOWN_UPPERCASE.contains(&upper.as_str()) {
                return upper;
            }
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Known abbreviations that should remain all-uppercase.
const KNOWN_UPPERCASE: &[&str] = &[
    "EC2", "ECS", "EKS", "RDS", "S3", "SQS", "SNS", "VPC", "IAM", "KMS", "WAF", "ELB", "CDN",
    "SQL", "VM", "DNS", "GKE", "GCS", "ACR", "AKS", "API", "CDK", "CLI", "SDK", "TLS", "SSL",
    "HTTP", "TCP", "UDP", // Kubernetes resource abbreviations
    "HPA", "PVC", "PV", "RBAC", "CRD",
];

/// Return an inline SVG `<g>` snippet for a recognizable service icon glyph
/// centred on (0,0) in a 16×16 coordinate space.
///
/// The snippets use `fill="currentColor"` / `stroke="currentColor"` so the
/// caller can apply a `color:` CSS property or a `fill=` attribute to tint them.
///
/// Returns `None` for services without a bespoke glyph; the caller falls back
/// to the plain service-name text label.
fn service_icon_svg(provider: CloudProvider, service: &str) -> Option<&'static str> {
    // Normalise the service name for lookup.
    let key = service.to_ascii_lowercase();
    match provider {
        CloudProvider::Aws => match key.as_str() {
            // EC2: simple server-rack outline — 3 stacked rectangles + 2 dots.
            "ec2" => Some(
                r#"<rect x="-7" y="-7" width="14" height="4" rx="1" fill="currentColor"/>
<rect x="-7" y="-1.5" width="14" height="4" rx="1" fill="currentColor"/>
<rect x="-7" y="4" width="14" height="4" rx="1" fill="currentColor"/>
<circle cx="4" cy="-5" r="1" fill="white"/>
<circle cx="4" cy="0.5" r="1" fill="white"/>
<circle cx="4" cy="6" r="1" fill="white"/>"#,
            ),
            // Lambda: λ symbol — two angled lines forming a Greek lambda.
            "lambda" => Some(
                r#"<path d="M-7,7 L-2,-1 L0,3" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M0,3 L7,-7" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/>"#,
            ),
            // S3: stylised bucket/cylinder.
            "s3" => Some(
                r#"<ellipse cx="0" cy="-5" rx="7" ry="2.5" fill="currentColor" opacity="0.9"/>
<rect x="-7" y="-5" width="14" height="10" fill="currentColor" opacity="0.6"/>
<ellipse cx="0" cy="5" rx="7" ry="2.5" fill="currentColor" opacity="0.9"/>"#,
            ),
            // RDS: stacked database cylinders.
            "rds" => Some(
                r#"<ellipse cx="0" cy="-4" rx="7" ry="2" fill="currentColor" opacity="0.85"/>
<rect x="-7" y="-4" width="14" height="5" fill="currentColor" opacity="0.6"/>
<ellipse cx="0" cy="1" rx="7" ry="2" fill="currentColor" opacity="0.85"/>
<rect x="-7" y="1" width="14" height="4" fill="currentColor" opacity="0.6"/>
<ellipse cx="0" cy="5" rx="7" ry="2" fill="currentColor" opacity="0.85"/>"#,
            ),
            // ELB: two nodes + branching lines representing load distribution.
            "elb" => Some(
                r#"<circle cx="0" cy="-5" r="3" fill="currentColor"/>
<circle cx="-6" cy="5" r="2.5" fill="currentColor"/>
<circle cx="6" cy="5" r="2.5" fill="currentColor"/>
<line x1="0" y1="-2" x2="-6" y2="2.5" stroke="currentColor" stroke-width="1.5"/>
<line x1="0" y1="-2" x2="6" y2="2.5" stroke="currentColor" stroke-width="1.5"/>"#,
            ),
            // CloudFront: globe + distribution ring.
            "cloudfront" => Some(
                r#"<circle cx="0" cy="0" r="7" fill="none" stroke="currentColor" stroke-width="1.8"/>
<ellipse cx="0" cy="0" rx="3.5" ry="7" fill="none" stroke="currentColor" stroke-width="1.2"/>
<line x1="-7" y1="0" x2="7" y2="0" stroke="currentColor" stroke-width="1.2"/>
<line x1="-6" y1="-3.5" x2="6" y2="-3.5" stroke="currentColor" stroke-width="0.8"/>
<line x1="-6" y1="3.5" x2="6" y2="3.5" stroke="currentColor" stroke-width="0.8"/>"#,
            ),
            // DynamoDB: stylised lightning bolt inside a shape (speed/NoSQL).
            "dynamodb" => Some(
                r#"<polygon points="-6,-7 6,-7 4,-1 7,-1 -4,7 -2,1 -7,1" fill="currentColor"/>"#,
            ),
            // IAM: person silhouette with a key.
            "iam" => Some(
                r#"<circle cx="0" cy="-4" r="3" fill="currentColor"/>
<path d="M-5,7 C-5,1 5,1 5,7" fill="currentColor"/>
<circle cx="4" cy="4" r="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/>
<line x1="4" y1="6.5" x2="4" y2="9" stroke="currentColor" stroke-width="1.5"/>
<line x1="4" y1="7.5" x2="6" y2="7.5" stroke="currentColor" stroke-width="1.5"/>"#,
            ),
            _ => None,
        },
        CloudProvider::Azure => match key.as_str() {
            // AzureVM: monitor + CPU symbol.
            "azurevm" | "vm" => Some(
                r#"<rect x="-7" y="-6" width="14" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.8"/>
<line x1="-4" y1="-3" x2="-4" y2="1" stroke="currentColor" stroke-width="1.2"/>
<line x1="0" y1="-4" x2="0" y2="2" stroke="currentColor" stroke-width="1.2"/>
<line x1="4" y1="-3" x2="4" y2="1" stroke="currentColor" stroke-width="1.2"/>
<line x1="-3" y1="4" x2="3" y2="4" stroke="currentColor" stroke-width="1.8"/>
<line x1="0" y1="4" x2="0" y2="7" stroke="currentColor" stroke-width="1.5"/>"#,
            ),
            // AzureFunction: lightning bolt (same as Lambda concept).
            "azurefunction" | "function" => Some(
                r#"<polygon points="-4,-8 2,-8 -1,-1 4,-1 -5,8 -1,0 -6,0" fill="currentColor"/>"#,
            ),
            // AzureBlobStorage: container with blob symbol.
            "azureblobstorage" | "blobstorage" => Some(
                r#"<rect x="-7" y="-7" width="14" height="14" rx="3" fill="none" stroke="currentColor" stroke-width="1.8"/>
<ellipse cx="-2" cy="-1" rx="3" ry="3.5" fill="currentColor" opacity="0.8"/>
<ellipse cx="3" cy="0" rx="2.5" ry="3" fill="currentColor" opacity="0.7"/>"#,
            ),
            // AzureSQL: cylinder (same as database concept).
            "azuresqldatabase" | "sqldatabase" | "sql" => Some(
                r#"<ellipse cx="0" cy="-4" rx="7" ry="2.5" fill="currentColor" opacity="0.9"/>
<rect x="-7" y="-4" width="14" height="10" fill="currentColor" opacity="0.6"/>
<ellipse cx="0" cy="6" rx="7" ry="2.5" fill="currentColor" opacity="0.9"/>"#,
            ),
            _ => None,
        },
        CloudProvider::Gcp => match key.as_str() {
            // ComputeEngine: server with GCP flavour — chip/circuit icon.
            "computeengine" => Some(
                r#"<rect x="-5" y="-5" width="10" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.8"/>
<rect x="-2.5" y="-2.5" width="5" height="5" fill="currentColor"/>
<line x1="-5" y1="-3" x2="-8" y2="-3" stroke="currentColor" stroke-width="1.5"/>
<line x1="-5" y1="3" x2="-8" y2="3" stroke="currentColor" stroke-width="1.5"/>
<line x1="5" y1="-3" x2="8" y2="-3" stroke="currentColor" stroke-width="1.5"/>
<line x1="5" y1="3" x2="8" y2="3" stroke="currentColor" stroke-width="1.5"/>
<line x1="-3" y1="-5" x2="-3" y2="-8" stroke="currentColor" stroke-width="1.5"/>
<line x1="3" y1="-5" x2="3" y2="-8" stroke="currentColor" stroke-width="1.5"/>"#,
            ),
            // CloudStorage: bucket icon.
            "cloudstorage" | "storage" => Some(
                r#"<path d="M-7,-3 L-4,-7 L4,-7 L7,-3 L7,7 L-7,7 Z" fill="currentColor" opacity="0.8"/>
<path d="M-7,-3 L7,-3" stroke="white" stroke-width="1.2"/>
<path d="M-5,-5 L-3,-8 M3,-8 L5,-5" stroke="white" stroke-width="1.2"/>"#,
            ),
            // BigQuery: magnifying glass + chart.
            "bigquery" => Some(
                r#"<circle cx="-1" cy="-1" r="5.5" fill="none" stroke="currentColor" stroke-width="1.8"/>
<line x1="3.5" y1="3.5" x2="7" y2="7" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"/>
<line x1="-3" y1="1" x2="1" y2="1" stroke="currentColor" stroke-width="1.2"/>
<line x1="-3" y1="-1" x2="1" y2="-1" stroke="currentColor" stroke-width="1.2"/>
<line x1="-3" y1="-3" x2="1" y2="-3" stroke="currentColor" stroke-width="1.2"/>"#,
            ),
            // CloudFunctions: function/lambda symbol in GCP style.
            "cloudfunctions" | "functions" => Some(
                r#"<path d="M-6,7 L-1,-2 L1,2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M1,2 L6,-7" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>"#,
            ),
            _ => None,
        },
        CloudProvider::Kubernetes => match key.as_str() {
            // Pod: circle with a dot — the Kubernetes pod symbol.
            "pod" => Some(
                r#"<circle cx="0" cy="0" r="7" fill="none" stroke="currentColor" stroke-width="2"/>
<circle cx="0" cy="0" r="2.5" fill="currentColor"/>"#,
            ),
            // Deployment: three overlapping squares — stacked replicas.
            "deployment" => Some(
                r#"<rect x="-6" y="-6" width="9" height="9" rx="1" fill="currentColor" opacity="0.5"/>
<rect x="-3" y="-3" width="9" height="9" rx="1" fill="currentColor" opacity="0.7"/>
<rect x="0" y="0" width="6" height="6" rx="1" fill="currentColor"/>"#,
            ),
            // Service: network circle with ports — Kubernetes service endpoints.
            "service" => Some(
                r#"<circle cx="0" cy="0" r="6" fill="none" stroke="currentColor" stroke-width="1.8"/>
<circle cx="0" cy="-6" r="1.8" fill="currentColor"/>
<circle cx="5.2" cy="3" r="1.8" fill="currentColor"/>
<circle cx="-5.2" cy="3" r="1.8" fill="currentColor"/>
<line x1="0" y1="-4.2" x2="0" y2="-1" stroke="currentColor" stroke-width="1"/>
<line x1="4" y1="2" x2="1.7" y2="0.5" stroke="currentColor" stroke-width="1"/>
<line x1="-4" y1="2" x2="-1.7" y2="0.5" stroke="currentColor" stroke-width="1"/>"#,
            ),
            // Ingress: arrow entering a gateway arch.
            "ingress" => Some(
                r#"<path d="M-7,0 L7,0" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
<path d="M3,-4 L7,0 L3,4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/>
<rect x="-7" y="-7" width="5" height="14" rx="2" fill="currentColor" opacity="0.7"/>"#,
            ),
            // ConfigMap: gear/cog representing configuration.
            "configmap" => Some(
                r#"<circle cx="0" cy="0" r="3.5" fill="none" stroke="currentColor" stroke-width="1.8"/>
<path d="M0,-8 L1,-5 L-1,-5 Z" fill="currentColor"/>
<path d="M0,8 L1,5 L-1,5 Z" fill="currentColor"/>
<path d="M-8,0 L-5,-1 L-5,1 Z" fill="currentColor"/>
<path d="M8,0 L5,-1 L5,1 Z" fill="currentColor"/>
<path d="M-5.7,-5.7 L-3.5,-3 L-4.5,-2 Z" fill="currentColor"/>
<path d="M5.7,5.7 L3.5,3 L4.5,2 Z" fill="currentColor"/>
<path d="M5.7,-5.7 L3,3.5 L2,4.5 Z" fill="currentColor"/>
<path d="M-5.7,5.7 L-3,-3.5 L-2,-4.5 Z" fill="currentColor"/>"#,
            ),
            _ => None,
        },
        CloudProvider::FontAwesome | CloudProvider::Devicons => None,
    }
}

/// Render the provider badge glyph (a small coloured square in the top-left
/// corner of the node) as inline SVG.
///
/// The badge is a 24×24 rounded rect at `(x + 4, y + 4)` filled with the
/// provider colour, containing a 2–3 character provider abbreviation in white.
pub fn render_cloud_badge(out: &mut String, provider: CloudProvider, x: i32, y: i32) {
    let bx = x + 4;
    let by = y + 4;
    let color = provider.header_fill();
    let label = provider.badge_label();
    let font_size = if label.len() <= 2 { 10 } else { 8 };

    out.push_str(&format!(
        "<rect class=\"cloud-icon-badge\" data-provider=\"{}\" \
         x=\"{bx}\" y=\"{by}\" width=\"24\" height=\"16\" rx=\"3\" ry=\"3\" \
         fill=\"{color}\"/>",
        escape_text(label),
    ));
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" dominant-baseline=\"middle\" \
         font-family=\"monospace\" font-size=\"{font_size}\" font-weight=\"700\" fill=\"#ffffff\">{lbl}</text>",
        bx + 12,
        by + 8,
        lbl = escape_text(label),
    ));
}

/// Render a complete cloud-icon box replacing the standard Object node box.
///
/// Layout:
/// ```text
/// ┌──────────────────────────────────┐  ← y
/// │ [AWS] EC2          (header band) │
/// ├──────────────────────────────────┤  ← y + header_h
/// │                                  │
/// │       label text                 │
/// │                                  │
/// └──────────────────────────────────┘  ← y + h
/// ```
#[allow(clippy::too_many_arguments)]
pub fn render_cloud_icon_box(
    out: &mut String,
    icon: &CloudIconStereotype,
    label: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_h: i32,
    node_id: &str,
) {
    let provider = icon.provider;
    let stroke = provider.stroke_color();
    let fill = provider.fill_color();
    let header_fill = provider.header_fill();
    let header_font_color = provider.header_font_color();

    // Outer box
    out.push_str(&format!(
        "<rect class=\"uml-node uml-cloud-icon-node\" \
         data-uml-id=\"{id}\" data-provider=\"{prov}\" data-service=\"{svc}\" \
         x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" ry=\"4\" \
         fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        id = escape_text(node_id),
        prov = escape_text(provider.badge_label()),
        svc = escape_text(&icon.service),
    ));

    // Header band
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"4\" ry=\"4\" \
         fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        hh = header_h,
    ));
    // Square off the bottom corners of the header (so it meets the body rect cleanly)
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{bty}\" width=\"{w}\" height=\"8\" fill=\"{header_fill}\"/>",
        bty = y + header_h - 8,
    ));
    // Header separator line
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{ly}\" x2=\"{x2}\" y2=\"{ly}\" \
         stroke=\"{stroke}\" stroke-width=\"1\"/>",
        ly = y + header_h,
        x2 = x + w,
    ));

    // Provider badge (top-left)
    render_cloud_badge(out, provider, x, y);

    // Service name in header (right of badge)
    let service_x = x + 34;
    let service_y = y + header_h / 2 + 4;
    let cx = x + w / 2;

    out.push_str(&format!(
        "<text x=\"{service_x}\" y=\"{service_y}\" \
         font-family=\"monospace\" font-size=\"10\" font-weight=\"700\" fill=\"{header_font_color}\" \
         data-cloud-service-name=\"true\">{svc}</text>",
        svc = escape_text(&icon.service),
    ));

    // Service icon glyph in body (if a bespoke SVG glyph exists for this service).
    let body_h = h - header_h;
    let glyph = service_icon_svg(provider, &icon.service);
    let has_glyph = glyph.is_some();
    let has_label = !label.is_empty() && label != node_id;
    let icon_zone_h = if has_glyph && has_label {
        // Share body: upper half for icon, lower half for label text.
        body_h / 2
    } else {
        body_h
    };
    let icon_cx = cx;
    let icon_cy = y + header_h + icon_zone_h / 2;

    if let Some(glyph_svg) = glyph {
        let icon_size = (icon_zone_h - 8).clamp(10, 24);
        let scale = icon_size as f64 / 16.0;
        out.push_str(&format!(
            "<g transform=\"translate({icon_cx},{icon_cy}) scale({scale:.3})\" \
             fill=\"{header_fill}\" stroke=\"{header_fill}\" opacity=\"0.82\" \
             data-cloud-service-icon=\"true\">{glyph_svg}</g>",
        ));
    }

    // Label text (body — lower portion when icon present, centred otherwise).
    let label_y = if has_glyph && has_label {
        y + header_h + icon_zone_h + (body_h - icon_zone_h) / 2 + 4
    } else {
        y + header_h + body_h / 2 + 4
    };
    if has_label {
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{label_y}\" text-anchor=\"middle\" \
             font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#0f172a\" \
             text-decoration=\"underline\" text-decoration-thickness=\"1\">{lbl}</text>",
            lbl = escape_text(label),
        ));
    }
}

/// Given a list of member strings from a `FamilyNode`, return the first
/// cloud icon stereotype detected, if any.
pub fn find_cloud_stereotype(members: &[crate::ast::ClassMember]) -> Option<CloudIconStereotype> {
    members
        .iter()
        .find_map(|m| parse_cloud_stereotype(m.text.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aws_ec2_stereotype() {
        let icon = parse_cloud_stereotype("<<aws-EC2>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::Aws);
        assert_eq!(icon.service, "EC2");
    }

    #[test]
    fn parses_aws_lambda_stereotype() {
        let icon = parse_cloud_stereotype("<<aws-Lambda>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::Aws);
        assert_eq!(icon.service, "Lambda");
    }

    #[test]
    fn parses_azure_vm_stereotype() {
        let icon = parse_cloud_stereotype("<<azure-vm>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::Azure);
        // "vm" is a known uppercase abbreviation → rendered as "VM"
        assert_eq!(icon.service, "VM");
    }

    #[test]
    fn parses_gcp_compute_engine_stereotype() {
        // Single camelCase word with no separator passes through as-is (no split point)
        let icon = parse_cloud_stereotype("<<gcp-ComputeEngine>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::Gcp);
        assert_eq!(icon.service, "ComputeEngine");
    }

    #[test]
    fn parses_font_awesome_stereotype_fa_prefix() {
        let icon = parse_cloud_stereotype("<<fa-cloud>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::FontAwesome);
        assert_eq!(icon.service, "Cloud");
    }

    #[test]
    fn parses_font_awesome5_stereotype_fa5_prefix() {
        let icon = parse_cloud_stereotype("<<fa5-cloud>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::FontAwesome);
        assert_eq!(icon.service, "Cloud");
    }

    #[test]
    fn parses_devicons_stereotype() {
        let icon = parse_cloud_stereotype("<<devicons_docker>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::Devicons);
        assert_eq!(icon.service, "Docker");
    }

    #[test]
    fn returns_none_for_non_cloud_stereotype() {
        assert!(parse_cloud_stereotype("<<component>>").is_none());
        assert!(parse_cloud_stereotype("<<interface>>").is_none());
        assert!(parse_cloud_stereotype("<<user>>").is_none());
    }

    #[test]
    fn returns_none_for_empty_provider() {
        // No service name after the prefix
        assert!(parse_cloud_stereotype("<<aws->>").is_none());
    }

    #[test]
    fn parses_k8s_pod_stereotype() {
        let icon = parse_cloud_stereotype("<<k8s-Pod>>").expect("should parse k8s-Pod");
        assert_eq!(icon.provider, CloudProvider::Kubernetes);
        assert_eq!(icon.service, "Pod");
    }

    #[test]
    fn parses_k8s_hpa_stereotype_preserves_uppercase() {
        let icon = parse_cloud_stereotype("<<k8s-HPA>>").expect("should parse k8s-HPA");
        assert_eq!(icon.provider, CloudProvider::Kubernetes);
        assert_eq!(icon.service, "HPA");
    }

    #[test]
    fn parses_k8s_statefulset_stereotype() {
        let icon = parse_cloud_stereotype("<<k8s-StatefulSet>>").expect("should parse");
        assert_eq!(icon.provider, CloudProvider::Kubernetes);
        assert_eq!(icon.service, "StatefulSet");
    }

    #[test]
    fn render_cloud_icon_box_k8s_has_blue_markers() {
        let icon = CloudIconStereotype {
            provider: CloudProvider::Kubernetes,
            service: "Pod".to_string(),
        };
        let mut out = String::new();
        render_cloud_icon_box(&mut out, &icon, "web-pod", 0, 0, 130, 80, 30, "wpod");

        assert!(out.contains("Pod"), "SVG must contain service name Pod");
        assert!(out.contains("K8s"), "SVG must contain K8s badge");
        assert!(
            out.contains("#326CE5"),
            "SVG must have Kubernetes blue stroke"
        );
        assert!(
            out.contains("data-provider="),
            "must have data-provider attribute"
        );
        assert!(
            out.contains("data-service="),
            "must have data-service attribute"
        );
    }

    #[test]
    fn provider_colors_are_distinct() {
        let providers = [
            CloudProvider::Aws,
            CloudProvider::Azure,
            CloudProvider::Gcp,
            CloudProvider::FontAwesome,
            CloudProvider::Devicons,
            CloudProvider::Kubernetes,
        ];
        // All stroke colors must be different
        let strokes: Vec<_> = providers.iter().map(|p| p.stroke_color()).collect();
        let unique: std::collections::BTreeSet<_> = strokes.iter().collect();
        assert_eq!(
            unique.len(),
            strokes.len(),
            "all stroke colors must be distinct"
        );

        // All fill colors must be different
        let fills: Vec<_> = providers.iter().map(|p| p.fill_color()).collect();
        let unique_fills: std::collections::BTreeSet<_> = fills.iter().collect();
        assert_eq!(
            unique_fills.len(),
            fills.len(),
            "all fill colors must be distinct"
        );
    }

    #[test]
    fn render_cloud_icon_box_includes_service_name_and_provider_markers() {
        let icon = CloudIconStereotype {
            provider: CloudProvider::Aws,
            service: "EC2".to_string(),
        };
        let mut out = String::new();
        render_cloud_icon_box(&mut out, &icon, "App Server", 10, 10, 120, 80, 24, "server");

        // Must include the service name
        assert!(out.contains("EC2"), "SVG must contain service name");
        // Must include the provider badge label
        assert!(out.contains("AWS"), "SVG must contain provider badge");
        // Must include the label text
        assert!(out.contains("App Server"), "SVG must contain node label");
        // Must have provider-specific stroke color
        assert!(out.contains("#FF9900"), "SVG must have AWS orange stroke");
        // Must have the cloud-icon-node class
        assert!(
            out.contains("uml-cloud-icon-node"),
            "SVG must have cloud-icon-node class"
        );
        // Must have data-provider attribute
        assert!(
            out.contains("data-provider="),
            "SVG must have data-provider attribute"
        );
        // Must have data-service attribute
        assert!(
            out.contains("data-service="),
            "SVG must have data-service attribute"
        );
    }

    #[test]
    fn render_cloud_icon_box_azure_has_distinct_markers() {
        let icon = CloudIconStereotype {
            provider: CloudProvider::Azure,
            service: "Blob Storage".to_string(),
        };
        let mut out = String::new();
        render_cloud_icon_box(&mut out, &icon, "Storage", 0, 0, 120, 80, 24, "storage");

        assert!(
            out.contains("Blob Storage"),
            "SVG must contain service name"
        );
        assert!(out.contains("Azure"), "SVG must contain Azure badge");
        assert!(out.contains("#0078D4"), "SVG must have Azure blue stroke");
    }

    #[test]
    fn render_cloud_icon_box_gcp_has_distinct_markers() {
        let icon = CloudIconStereotype {
            provider: CloudProvider::Gcp,
            service: "BigQuery".to_string(),
        };
        let mut out = String::new();
        render_cloud_icon_box(&mut out, &icon, "Analytics", 0, 0, 120, 80, 24, "bq");

        assert!(out.contains("BigQuery"), "SVG must contain service name");
        assert!(out.contains("GCP"), "SVG must contain GCP badge");
        assert!(out.contains("#4285F4"), "SVG must have GCP blue stroke");
    }

    #[test]
    fn humanise_service_name_handles_known_abbreviations() {
        // EC2 stays EC2, not Ec2
        assert_eq!(humanise_service_name("EC2"), "EC2");
        // Multi-word names
        assert_eq!(humanise_service_name("blob_storage"), "Blob Storage");
        assert_eq!(humanise_service_name("compute-engine"), "Compute Engine");
    }

    // ── Service icon glyph tests (#1500) ────────────────────────────────────

    #[test]
    fn service_icon_svg_returns_some_for_known_aws_services() {
        // Core AWS services must have bespoke glyphs.
        assert!(
            service_icon_svg(CloudProvider::Aws, "EC2").is_some(),
            "EC2 must have a glyph"
        );
        assert!(
            service_icon_svg(CloudProvider::Aws, "Lambda").is_some(),
            "Lambda must have a glyph"
        );
        assert!(
            service_icon_svg(CloudProvider::Aws, "S3").is_some(),
            "S3 must have a glyph"
        );
    }

    #[test]
    fn service_icon_svg_returns_some_for_known_k8s_resources() {
        assert!(
            service_icon_svg(CloudProvider::Kubernetes, "Pod").is_some(),
            "Pod must have a glyph"
        );
        assert!(
            service_icon_svg(CloudProvider::Kubernetes, "Service").is_some(),
            "Service must have a glyph"
        );
        assert!(
            service_icon_svg(CloudProvider::Kubernetes, "Deployment").is_some(),
            "Deployment must have a glyph"
        );
    }

    #[test]
    fn service_icon_svg_returns_none_for_unknown_services() {
        assert!(
            service_icon_svg(CloudProvider::Aws, "SomeFutureService").is_none(),
            "unknown services must return None"
        );
        assert!(
            service_icon_svg(CloudProvider::Kubernetes, "CustomCRD").is_none(),
            "unknown k8s resource must return None"
        );
    }

    #[test]
    fn render_cloud_icon_box_ec2_embeds_service_glyph() {
        let icon = CloudIconStereotype {
            provider: CloudProvider::Aws,
            service: "EC2".to_string(),
        };
        let mut out = String::new();
        render_cloud_icon_box(&mut out, &icon, "Web Server", 0, 0, 140, 90, 30, "web");

        // The SVG should contain the service icon glyph group.
        assert!(
            out.contains("data-cloud-service-icon"),
            "rendered EC2 box must embed a service icon glyph"
        );
        // And the label text.
        assert!(out.contains("Web Server"), "must contain label");
        // And the EC2 service name in header.
        assert!(out.contains("EC2"), "must contain service name in header");
    }

    #[test]
    fn render_cloud_icon_box_k8s_pod_embeds_glyph_and_k8s_badge() {
        let icon = CloudIconStereotype {
            provider: CloudProvider::Kubernetes,
            service: "Pod".to_string(),
        };
        let mut out = String::new();
        render_cloud_icon_box(&mut out, &icon, "api-pod", 0, 0, 140, 90, 30, "apod");

        assert!(
            out.contains("data-cloud-service-icon"),
            "k8s Pod box must embed service icon glyph"
        );
        assert!(out.contains("K8s"), "k8s badge must be K8s");
        assert!(
            out.contains("#326CE5"),
            "k8s stroke must be Kubernetes blue"
        );
    }
}
