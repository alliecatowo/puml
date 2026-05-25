use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdlibAlias {
    pub slug: &'static str,
    pub target: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdlibEntry {
    /// Logical include path users can reach through `!include <...>`.
    pub path: String,
    /// On-disk path under the local `stdlib/` directory.
    pub physical_path: String,
    /// True when `path` is an alias for a different physical path.
    pub alias: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdlibPackSummary {
    pub name: String,
    pub status: StdlibPackStatus,
    pub files: usize,
    pub aliases: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdlibPackStatus {
    Available,
    Builtin,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdlibBuiltinIncludeKind {
    OpenIconicSprite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdlibBuiltinInclude {
    pub logical_path: String,
    pub pack: String,
    pub symbol: String,
    pub kind: StdlibBuiltinIncludeKind,
    pub content: String,
}

pub const STDLIB_ALIASES: &[StdlibAlias] = &[
    StdlibAlias {
        slug: "awslib",
        target: "awslib14",
    },
    // PlantUML's upstream docs still point older Material sprite users at
    // material2.1.19; keep those diagrams resolving to our deterministic
    // material compatibility subset.
    StdlibAlias {
        slug: "material2",
        target: "material",
    },
    StdlibAlias {
        slug: "material2.1.19",
        target: "material",
    },
];

pub const MISSING_UPSTREAM_STDLIB_PACKS: &[&str] = &[
    "ada",
    "archimate",
    "aws",
    "bootstrap",
    "classy",
    "classy-c4",
    "DomainStory",
    "edgy",
    "eip",
    "elastic",
    "k8s",
    "material7",
];

pub const BUILTIN_STDLIB_PACKS: &[&str] = &["openiconic"];

pub fn apply_stdlib_path_alias(path: PathBuf) -> PathBuf {
    let mut components = path.components();
    let Some(first) = components.next().and_then(|component| match component {
        std::path::Component::Normal(name) => Some(name),
        _ => None,
    }) else {
        return path;
    };
    let Some(alias) = STDLIB_ALIASES.iter().find(|alias| first == alias.slug) else {
        return path;
    };

    let mut aliased = PathBuf::from(alias.target);
    for component in components {
        aliased.push(component.as_os_str());
    }
    aliased
}

pub fn resolve_local_stdlib_root(include_root: Option<&Path>) -> Result<PathBuf, String> {
    if let Ok(env_root) = std::env::var("PUML_STDLIB_ROOT") {
        if let Some(root) = canonical_stdlib_dir(PathBuf::from(env_root)) {
            return Ok(root);
        }
    }

    if let Some(root) = include_root {
        if let Some(root) = canonical_stdlib_dir(root.join("stdlib")) {
            return Ok(root);
        }
    }

    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        if let Some(root) = canonical_stdlib_dir(PathBuf::from(manifest_dir).join("stdlib")) {
            return Ok(root);
        }
    }

    Err(
        "no local stdlib directory found; set PUML_STDLIB_ROOT or run from a checkout with stdlib/"
            .to_string(),
    )
}

fn canonical_stdlib_dir(path: PathBuf) -> Option<PathBuf> {
    let canon = path.canonicalize().ok()?;
    canon.is_dir().then_some(canon)
}

pub fn local_stdlib_inventory(include_root: Option<&Path>) -> Result<Vec<StdlibEntry>, String> {
    let root = resolve_local_stdlib_root(include_root)?;
    inventory_from_root(&root)
}

pub fn inventory_from_root(root: &Path) -> Result<Vec<StdlibEntry>, String> {
    let mut physical_paths = Vec::new();
    collect_puml_files(root, root, &mut physical_paths)?;
    physical_paths.sort();

    let mut entries = Vec::new();
    for physical_path in physical_paths {
        entries.push(StdlibEntry {
            path: physical_path.clone(),
            physical_path,
            alias: false,
        });
    }

    let direct_entries = entries.clone();
    for alias in STDLIB_ALIASES {
        let target_prefix = format!("{}/", alias.target);
        for entry in direct_entries.iter() {
            if let Some(rest) = entry.physical_path.strip_prefix(&target_prefix) {
                entries.push(StdlibEntry {
                    path: format!("{}/{}", alias.slug, rest),
                    physical_path: entry.physical_path.clone(),
                    alias: true,
                });
            }
        }
    }

    entries.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.physical_path.cmp(&b.physical_path))
            .then_with(|| a.alias.cmp(&b.alias))
    });
    Ok(entries)
}

pub fn stdlib_pack_summaries(entries: &[StdlibEntry]) -> Vec<StdlibPackSummary> {
    let mut packs: std::collections::BTreeMap<String, StdlibPackSummary> =
        std::collections::BTreeMap::new();
    for entry in entries {
        let Some(pack_name) = stdlib_path_pack(&entry.path) else {
            continue;
        };
        let pack = packs
            .entry(pack_name.to_string())
            .or_insert_with(|| StdlibPackSummary {
                name: pack_name.to_string(),
                status: StdlibPackStatus::Available,
                files: 0,
                aliases: 0,
            });
        if entry.alias {
            pack.aliases += 1;
        } else {
            pack.files += 1;
        }
    }

    for pack in sorted_missing_stdlib_packs() {
        packs
            .entry(pack.to_string())
            .or_insert_with(|| StdlibPackSummary {
                name: pack.to_string(),
                status: StdlibPackStatus::Unavailable,
                files: 0,
                aliases: 0,
            });
    }
    for pack in sorted_builtin_stdlib_packs() {
        packs
            .entry(pack.to_string())
            .or_insert_with(|| StdlibPackSummary {
                name: pack.to_string(),
                status: StdlibPackStatus::Builtin,
                files: 0,
                aliases: 0,
            });
    }

    packs.into_values().collect()
}

pub fn available_stdlib_packs(entries: &[StdlibEntry]) -> Vec<String> {
    stdlib_pack_summaries(entries)
        .into_iter()
        .filter(|pack| {
            matches!(
                pack.status,
                StdlibPackStatus::Available | StdlibPackStatus::Builtin
            )
        })
        .map(|pack| pack.name)
        .collect()
}

pub fn sorted_builtin_stdlib_packs() -> Vec<&'static str> {
    let mut packs = BUILTIN_STDLIB_PACKS.to_vec();
    packs.sort_unstable();
    packs.dedup();
    packs
}

pub fn sorted_missing_stdlib_packs() -> Vec<&'static str> {
    let mut packs = MISSING_UPSTREAM_STDLIB_PACKS.to_vec();
    packs.sort_unstable();
    packs.dedup();
    packs
}

pub fn stdlib_path_pack(path: &str) -> Option<&str> {
    path.split('/')
        .next()
        .filter(|pack| !pack.is_empty() && *pack != "." && *pack != "..")
}

pub fn is_known_missing_stdlib_pack(pack: &str) -> bool {
    MISSING_UPSTREAM_STDLIB_PACKS
        .iter()
        .any(|missing| missing.eq_ignore_ascii_case(pack))
}

pub fn is_builtin_stdlib_pack(pack: &str) -> bool {
    BUILTIN_STDLIB_PACKS
        .iter()
        .any(|builtin| builtin.eq_ignore_ascii_case(pack))
}

pub fn resolve_builtin_stdlib_include(path: &Path) -> Option<StdlibBuiltinInclude> {
    let logical_path = path_to_slash_string(path);
    let pack = stdlib_path_pack(&logical_path)?;
    if !pack.eq_ignore_ascii_case("openiconic") {
        return None;
    }

    let icon_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())?;
    let (symbol, svg) = crate::sprites::openiconic_svg_source(icon_name)?;
    let content = format!(
        "' synthetic stdlib include <{logical_path}> resolved from built-in OpenIconic icons\nsprite ${symbol} {svg}\n"
    );
    Some(StdlibBuiltinInclude {
        logical_path,
        pack: "openiconic".to_string(),
        symbol,
        kind: StdlibBuiltinIncludeKind::OpenIconicSprite,
        content,
    })
}

fn collect_puml_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let mut children = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read stdlib directory '{}': {e}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("failed to read stdlib directory '{}': {e}", dir.display()))?;
    children.sort_by_key(|entry| entry.file_name());

    for child in children {
        let path = child.path();
        let file_type = child.file_type().map_err(|e| {
            format!(
                "failed to inspect stdlib path '{}': {e}",
                child.path().display()
            )
        })?;
        if file_type.is_dir() {
            collect_puml_files(root, &path, out)?;
            continue;
        }
        if !file_type.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("puml") {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map_err(|e| format!("failed to relativize stdlib path '{}': {e}", path.display()))?;
        out.push(path_to_slash_string(rel));
    }
    Ok(())
}

fn path_to_slash_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn stdlib_paths_json(entries: &[StdlibEntry]) -> String {
    let paths = entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    serde_json::to_string(&paths).unwrap_or_else(|_| "[]".to_string())
}

pub fn format_stdlib_listing(root: &Path, entries: &[StdlibEntry]) -> String {
    let mut out = String::new();
    out.push_str("# PUML local stdlib inventory (deterministic shim subset; not full upstream plantuml-stdlib)\n");
    out.push_str(&format!("# root: {}\n", root.display()));
    for alias in STDLIB_ALIASES {
        out.push_str(&format!("# alias: {} -> {}\n", alias.slug, alias.target));
    }
    out.push_str(&format!(
        "# missing upstream packs: {}\n",
        sorted_missing_stdlib_packs().join(", ")
    ));
    out.push_str(&format!(
        "# builtin packs: {}\n",
        sorted_builtin_stdlib_packs().join(", ")
    ));
    for entry in entries {
        if entry.alias {
            out.push_str(&format!("{} -> {}\n", entry.path, entry.physical_path));
        } else {
            out.push_str(&entry.path);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_includes_alias_paths_deterministically() {
        let root = resolve_local_stdlib_root(None).expect("stdlib root");
        let entries = inventory_from_root(&root).expect("inventory");
        let paths = entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>();

        assert!(paths.contains(&"awslib/Compute/EC2.puml"));
        assert!(paths.contains(&"awslib14/Compute/EC2.puml"));
        assert!(paths.contains(&"material2/folder.puml"));
        assert!(paths.contains(&"material2.1.19/folder.puml"));
        assert!(paths.contains(&"material/folder.puml"));

        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn builtin_openiconic_include_resolves_without_filesystem_entry() {
        let builtin = resolve_builtin_stdlib_include(Path::new("openiconic/folder.puml"))
            .expect("openiconic folder should resolve as built-in include");

        assert_eq!(builtin.pack, "openiconic");
        assert_eq!(builtin.symbol, "folder");
        assert_eq!(builtin.kind, StdlibBuiltinIncludeKind::OpenIconicSprite);
        assert!(builtin.content.contains("sprite $folder <svg"));
    }

    #[test]
    fn pack_summaries_classify_builtin_and_missing_packs_separately() {
        let root = resolve_local_stdlib_root(None).expect("stdlib root");
        let entries = inventory_from_root(&root).expect("inventory");
        let packs = stdlib_pack_summaries(&entries);

        let openiconic = packs
            .iter()
            .find(|pack| pack.name == "openiconic")
            .expect("openiconic pack summary");
        assert_eq!(openiconic.status, StdlibPackStatus::Builtin);

        let bootstrap = packs
            .iter()
            .find(|pack| pack.name == "bootstrap")
            .expect("bootstrap pack summary");
        assert_eq!(bootstrap.status, StdlibPackStatus::Unavailable);
    }
}
