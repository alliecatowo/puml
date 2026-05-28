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
    "HTTP", "TCP", "UDP",
];

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

    // Label text (body center)
    let label_y = y + header_h + (h - header_h) / 2 + 4;
    if !label.is_empty() && label != node_id {
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
    fn provider_colors_are_distinct() {
        let providers = [
            CloudProvider::Aws,
            CloudProvider::Azure,
            CloudProvider::Gcp,
            CloudProvider::FontAwesome,
            CloudProvider::Devicons,
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
}
