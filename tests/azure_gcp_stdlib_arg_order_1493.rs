//! Regression tests for Azure and GCP cloud macro arg-order bug (issue #1493).
//!
//! `AzureIcon($alias, $service, $label)` and `GCPIcon($alias, $service, $label)`
//! previously had swapped parameter names that caused the macro name (e.g. `AzureVM`)
//! to appear as the underlined display label and the user-supplied label (e.g. `"Web VM"`)
//! to be stuffed into the stereotype suffix (with literal quotes).
//!
//! After the fix:
//! - The user-supplied label is the underlined body text (`text-decoration="underline"`).
//! - The service name (e.g. `AzureVM`, `ComputeEngine`) is the header band text
//!   (`data-cloud-service-name="true"`).
//! - The stereotype is well-formed: `<<azure-AzureVM>>` / `<<gcp-ComputeEngine>>`.

use assert_cmd::Command;

fn fixture(path: &str) -> String {
    format!("{}/tests/fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

/// Azure: user-supplied label appears underlined; service name appears in header band.
#[test]
fn azure_vm_user_label_is_body_text_not_macro_name() {
    let source = std::fs::read_to_string(fixture("stdlib_catalog/valid_azure_services.puml"))
        .expect("azure fixture");

    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "svg", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(svg).expect("utf8 svg");

    // User-supplied label "Worker VM" must be the underlined body text.
    assert!(
        svg.contains(">Worker VM<"),
        "Azure user label 'Worker VM' should appear as body text: check svg for text-decoration=\"underline\">"
    );
    // The service name must appear in the header band, not as the underlined label.
    assert!(
        svg.contains("data-cloud-service-name=\"true\">AzureVM<"),
        "AzureVM service name should appear in the header band (data-cloud-service-name): {svg}"
    );
    // The macro name must NOT appear as the underlined display label.
    assert!(
        !svg.contains("text-decoration=\"underline\" text-decoration-thickness=\"1\">AzureVM<"),
        "AzureVM macro name must NOT be the underlined label — that indicates the arg-order bug is present"
    );
}

/// Azure: all service shims produce correctly-stereotyped objects (service in header, label in body).
#[test]
fn azure_all_service_shims_label_body_not_header() {
    let source = std::fs::read_to_string(fixture("stdlib_catalog/valid_azure_services.puml"))
        .expect("azure fixture");

    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "svg", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(svg).expect("utf8 svg");

    // All user-supplied labels from the fixture must appear as underlined body text.
    for label in &[
        "ProcessOrder",
        "Orders DB",
        "Events Store",
        "Worker VM",
        "Media Storage",
        "App Cluster",
        "Secrets",
        "Message Bus",
    ] {
        assert!(
            svg.contains(&format!(">{label}<")),
            "Azure user label '{label}' should appear as body text in SVG"
        );
    }

    // All service names must appear as header-band text, not as underlined labels.
    for service in &[
        "AzureFunction",
        "AzureSQLDatabase",
        "AzureCosmosDB",
        "AzureVM",
        "AzureBlobStorage",
        "AzureKubernetesService",
        "AzureKeyVault",
        "AzureServiceBus",
    ] {
        assert!(
            svg.contains(&format!("data-cloud-service-name=\"true\">{service}<")),
            "Azure service name '{service}' should be in the header band (data-cloud-service-name)"
        );
        assert!(
            !svg.contains(&format!(
                "text-decoration=\"underline\" text-decoration-thickness=\"1\">{service}<"
            )),
            "Azure service name '{service}' must NOT be the underlined body label"
        );
    }
}

/// GCP: user-supplied label appears underlined; service name appears in header band.
#[test]
fn gcp_compute_engine_user_label_is_body_text_not_macro_name() {
    let source = std::fs::read_to_string(fixture("stdlib_catalog/valid_gcp_services.puml"))
        .expect("gcp fixture");

    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "svg", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(svg).expect("utf8 svg");

    // User-supplied label "App VM" must be the underlined body text.
    assert!(
        svg.contains(">App VM<"),
        "GCP user label 'App VM' should appear as body text"
    );
    // The service name must appear in the header band, not as the underlined label.
    assert!(
        svg.contains("data-cloud-service-name=\"true\">ComputeEngine<"),
        "ComputeEngine service name should appear in header band (data-cloud-service-name): {svg}"
    );
    // The macro name must NOT appear as the underlined display label.
    assert!(
        !svg.contains(
            "text-decoration=\"underline\" text-decoration-thickness=\"1\">ComputeEngine<"
        ),
        "ComputeEngine macro name must NOT be the underlined label — arg-order bug present"
    );
}

/// GCP: all service shims produce correctly-stereotyped objects.
#[test]
fn gcp_all_service_shims_label_body_not_header() {
    let source = std::fs::read_to_string(fixture("stdlib_catalog/valid_gcp_services.puml"))
        .expect("gcp fixture");

    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "svg", "-"])
        .write_stdin(source)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(svg).expect("utf8 svg");

    // All user-supplied labels from the fixture must appear as underlined body text.
    for label in &[
        "App VM",
        "Data Lake",
        "Analytics",
        "App DB",
        "Thumbnail Gen",
        "Services Cluster",
        "Event Bus",
        "Global LB",
    ] {
        assert!(
            svg.contains(&format!(">{label}<")),
            "GCP user label '{label}' should appear as body text in SVG"
        );
    }

    // All service names must appear as header-band text, not as underlined labels.
    for service in &[
        "ComputeEngine",
        "CloudStorage",
        "BigQuery",
        "CloudSQL",
        "CloudFunctions",
        "GKE",
        "PubSub",
        "CloudLoadBalancing",
    ] {
        assert!(
            svg.contains(&format!("data-cloud-service-name=\"true\">{service}<")),
            "GCP service name '{service}' should be in the header band (data-cloud-service-name)"
        );
        assert!(
            !svg.contains(&format!(
                "text-decoration=\"underline\" text-decoration-thickness=\"1\">{service}<"
            )),
            "GCP service name '{service}' must NOT be the underlined body label"
        );
    }
}

/// Stereotype suffix is well-formed: `azure-<ServiceName>` (no literal quotes from label).
#[test]
fn azure_stereotype_suffix_contains_service_name_not_user_label() {
    let src = "@startuml\n\
               !include <azure/AzureVM>\n\
               AzureVM(vm1, \"Web VM\", \"Linux\")\n\
               @enduml\n";

    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "svg", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(svg).expect("utf8 svg");

    // The cloud-icon-node must carry data-service="AzureVM" (the service, not the user label).
    assert!(
        svg.contains("data-service=\"AzureVM\""),
        "Node data-service attribute should be 'AzureVM', not the user label: {svg}"
    );
    // The user label "Web VM" must appear as underlined body text.
    assert!(
        svg.contains(">Web VM<"),
        "User label 'Web VM' must appear as body text: {svg}"
    );
    // Literal quotes must not appear in the service attribute (pre-fix symptom).
    assert!(
        !svg.contains("data-service=\"&quot;"),
        "data-service must not contain escaped quotes — arg-order bug present: {svg}"
    );
}

/// GCP stereotype suffix is well-formed.
#[test]
fn gcp_stereotype_suffix_contains_service_name_not_user_label() {
    let src = "@startuml\n\
               !include <gcp/ComputeEngine>\n\
               ComputeEngine(vm1, \"API Server\", \"n2-standard-4\")\n\
               @enduml\n";

    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "svg", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(svg).expect("utf8 svg");

    // The cloud-icon-node must carry data-service="ComputeEngine".
    assert!(
        svg.contains("data-service=\"ComputeEngine\""),
        "Node data-service should be 'ComputeEngine', not the user label: {svg}"
    );
    // User label must appear as underlined body text.
    assert!(
        svg.contains(">API Server<"),
        "User label 'API Server' must appear as body text: {svg}"
    );
    // Literal quotes must not appear in the service attribute.
    assert!(
        !svg.contains("data-service=\"&quot;"),
        "data-service must not contain escaped quotes — arg-order bug present: {svg}"
    );
}
