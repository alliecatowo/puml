use crate::cli::{AboutArgs, AboutFormat};
use serde::Serialize;

/// Build and runtime information printed by `puml about`.
#[derive(Debug, Serialize)]
pub struct AboutInfo {
    pub version: String,
    pub profile: String,
    pub arch: String,
    pub families: Vec<String>,
    #[serde(rename = "stdlibPath")]
    pub stdlib_path: Option<String>,
}

/// Collect build and runtime information.
pub fn collect_about_info() -> AboutInfo {
    let profile = if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    };

    AboutInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        profile,
        arch: std::env::consts::ARCH.to_string(),
        families: puml::DiagramFamily::all_known()
            .iter()
            .map(|f| f.as_str().to_string())
            .collect(),
        stdlib_path: std::env::var("PUML_STDLIB_PATH").ok(),
    }
}

fn format_families_line(families: &[String]) -> String {
    families.join(", ")
}

fn format_human(info: &AboutInfo) -> String {
    let families_line = format_families_line(&info.families);
    let stdlib = info.stdlib_path.as_deref().unwrap_or("(bundled)");

    format!(
        "puml {version}\nprofile : {profile}\narch    : {arch}\nfamilies: {families}\nstdlib  : {stdlib}",
        version = info.version,
        profile = info.profile,
        arch = info.arch,
        families = families_line,
        stdlib = stdlib,
    )
}

/// Run the `about` subcommand. Returns an exit code (0 = success).
pub fn run_about(args: &AboutArgs) -> Result<i32, String> {
    let info = collect_about_info();

    match args.format {
        AboutFormat::Human => {
            println!("{}", format_human(&info));
        }
        AboutFormat::Json => {
            let json = serde_json::to_string_pretty(&info)
                .map_err(|e| format!("failed to serialize about info: {e}"))?;
            println!("{json}");
        }
    }

    Ok(0)
}
