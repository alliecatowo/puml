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
        MISSING_UPSTREAM_STDLIB_PACKS.join(", ")
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
}
