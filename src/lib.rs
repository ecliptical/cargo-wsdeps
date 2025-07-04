use cargo_metadata::{Dependency, Package, camino::Utf8PathBuf};
use cargo_toml::Workspace;
use std::collections::{BTreeMap, BTreeSet, HashSet};

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
pub type PartitionedDependencies = (BTreeMap<String, Vec<MemberDependency>>, BTreeSet<String>);

pub fn partition_dependencies(
    workspace: &Workspace,
    selected: &[&Package],
) -> anyhow::Result<PartitionedDependencies> {
    let current_deps: HashSet<_> = workspace.dependencies.keys().cloned().collect();
    let mut needed_deps: BTreeMap<_, _> = BTreeMap::new();

    for &member in selected {
        for dep in member.dependencies.iter().filter(|&dep| dep.path.is_none()) {
            needed_deps
                .entry(dep.name.clone())
                .or_insert_with(|| Vec::with_capacity(1))
                .push(MemberDependency {
                    name: member.name.to_string(),
                    manifest_path: member.manifest_path.clone(),
                    dependency: dep.clone(),
                });
        }
    }

    needed_deps.retain(|_, members| {
        members.sort_by(|a, b| a.name.cmp(&b.name));
        members.len() > 1
    });

    let (common, remove) = current_deps
        .into_iter()
        .partition::<BTreeSet<_>, _>(|name| needed_deps.contains_key(name));

    needed_deps.retain(|name, _| !common.contains(name));

    Ok((needed_deps, remove))
}
