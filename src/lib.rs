use cargo_metadata::{Dependency, Package, camino::Utf8PathBuf};
use cargo_toml::{Manifest, Workspace};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;

pub mod diff;
pub mod show;

/// Describes the dependency of a workspace member crate.
#[derive(Debug, Clone)]
pub struct MemberDependency {
    /// Member crate name.
    pub name: String,
    /// Path to the manifest file of the member crate.
    pub manifest_path: Utf8PathBuf,
    /// Dependency of the member crate.
    pub dependency: Dependency,
}

/// Depenndencies to add to and remove from the workspace.
pub type PartitionedDependencies = (
    BTreeMap<String, Vec<MemberDependency>>,
    BTreeSet<String>,
    BTreeMap<String, MemberDependency>,
);

pub fn partition_dependencies(
    workspace: &Workspace,
    selected: &[&Package],
    aggressive: bool,
) -> anyhow::Result<PartitionedDependencies> {
    let current_deps: HashSet<_> = workspace.dependencies.keys().cloned().collect();

    // Members of each dep, split by how the member declares it:
    //   - `needed_deps`: declared inline (`foo = "1"` or `foo = { version = ... }`)
    //   - `ws_users`: declared as `foo.workspace = true`
    // cargo_metadata flattens dependencies and does not expose the `workspace`
    // flag, so we re-parse each member manifest with cargo_toml to detect it.
    let mut needed_deps: BTreeMap<String, Vec<MemberDependency>> = BTreeMap::new();
    let mut ws_users: BTreeMap<String, Vec<MemberDependency>> = BTreeMap::new();

    for &member in selected {
        let content = fs::read_to_string(member.manifest_path.as_std_path())?;
        let member_manifest: Manifest = Manifest::from_str(&content)?;
        let mut inherited: HashSet<String> = HashSet::new();
        for (name, dep) in member_manifest
            .dependencies
            .iter()
            .chain(member_manifest.dev_dependencies.iter())
            .chain(member_manifest.build_dependencies.iter())
        {
            if matches!(dep, cargo_toml::Dependency::Inherited(_)) {
                inherited.insert(name.clone());
            }
        }

        for dep in member.dependencies.iter().filter(|&dep| dep.path.is_none()) {
            let md = MemberDependency {
                name: member.name.to_string(),
                manifest_path: member.manifest_path.clone(),
                dependency: dep.clone(),
            };
            if inherited.contains(&dep.name) {
                ws_users.entry(dep.name.clone()).or_default().push(md);
            } else {
                needed_deps.entry(dep.name.clone()).or_default().push(md);
            }
        }
    }

    // Sort inline users for stable output, then decide which inline members
    // need to be converted to `workspace = true`. A dep is worth sharing only
    // when 2+ members use it in total (inline + already-inherited), and there
    // must be at least one inline holdout to convert.
    needed_deps.retain(|name, members| {
        members.sort_by(|a, b| a.name.cmp(&b.name));
        let inherited_count = ws_users.get(name).map(|v| v.len()).unwrap_or(0);
        !members.is_empty() && (members.len() + inherited_count) > 1
    });

    // A workspace dep is kept if at least one member still references it
    // (either inline as a holdout, or via `workspace = true`).
    let (common, mut remove) = current_deps
        .into_iter()
        .partition::<BTreeSet<_>, _>(|name| {
            needed_deps.contains_key(name) || ws_users.contains_key(name)
        });

    // --aggressive: move a workspace dep back into the sole member that uses
    // it, but only when exactly one member inherits it and no other member
    // references it inline (otherwise consolidation would re-add it).
    let mut inline: BTreeMap<String, MemberDependency> = BTreeMap::new();
    if aggressive {
        for name in &common {
            let inherited_count = ws_users.get(name).map(|v| v.len()).unwrap_or(0);
            let inline_count = needed_deps.get(name).map(|v| v.len()).unwrap_or(0);
            if inherited_count == 1
                && inline_count == 0
                && let Some(mut users) = ws_users.remove(name)
                && let Some(md) = users.pop()
            {
                inline.insert(name.clone(), md);
                remove.insert(name.clone());
            }
        }
    }

    Ok((needed_deps, remove, inline))
}
