use cargo_metadata::{
    Dependency, DependencyKind, Metadata,
    camino::{Utf8Path, Utf8PathBuf},
};
use patcher::{DiffAlgorithm, Differ, MultifilePatch};
use pathdiff::diff_utf8_paths;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::read_to_string,
};
use toml_edit::{Array, DocumentMut, InlineTable, Table, table, value};

use crate::MemberDependency;

pub fn generate_diff(
    add: &BTreeMap<String, Vec<MemberDependency>>,
    remove: &BTreeSet<String>,
    metadata: &Metadata,
    dotted: bool,
) -> anyhow::Result<()> {
    let mut changes = Vec::with_capacity(add.len() + 1);

    let workspace_path = metadata.workspace_root.join("Cargo.toml");
    let workspace_content = read_to_string(&workspace_path)?;
    let mut workspace_doc: DocumentMut = workspace_content.parse()?;

    if let Some(workspace_dependencies) =
        workspace_doc["workspace"]["dependencies"].as_table_like_mut()
    {
        for name in remove {
            workspace_dependencies.remove(name);
        }
    }

    if !add.is_empty() {
        let Some(workspace_dependencies) = workspace_doc["workspace"]["dependencies"]
            .or_insert(table())
            .as_table_mut()
        else {
            anyhow::bail!("Invalid workspace dependencies entry");
        };

        let mut member_deps = BTreeMap::new();

        for (name, members) in add {
            // TODO determine how to reconcile version
            let mut dependency = None;
            let mut no_default_features = false;
            let mut features = BTreeSet::new();
            for member in members {
                if dependency.is_none() {
                    // TODO we can't just grab the first one
                    dependency = Some(member.dependency.clone());
                }

                features.extend(member.dependency.features.iter().cloned());
                no_default_features |= !member.dependency.uses_default_features;

                member_deps
                    .entry(member.manifest_path.clone())
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(member.dependency.clone());
            }

            if let Some(dependency) = dependency {
                let value = if no_default_features || !features.is_empty() {
                    let mut entry = InlineTable::new();
                    entry.insert("version", dependency.req.to_string().into());

                    if no_default_features {
                        entry.insert("default-features", false.into());
                    }

                    if !features.is_empty() {
                        entry.insert("features", Array::from_iter(features).into());
                    }

                    entry.into()
                } else {
                    value(dependency.req.to_string())
                };

                workspace_dependencies.insert(name, value);
            }
        }

        for (path, dependencies) in member_deps {
            update_member(&path, &dependencies, dotted, &mut changes)?;
        }
    }

    changes.push((workspace_path, workspace_content, workspace_doc.to_string()));

    let mut patches = Vec::new();

    for (path, original, modified) in changes {
        let differ = Differ::new(&original, &modified);
        let mut patch = differ.generate();

        let relative_path = diff_utf8_paths(&path, &metadata.workspace_root).unwrap_or(path);
        patch.old_file = relative_path.to_string();
        patch.new_file = relative_path.to_string();

        patches.push(patch);
    }

    let multi_patch = MultifilePatch::new(patches);

    println!("{multi_patch}");
    Ok(())
}

fn update_member(
    path: &Utf8Path,
    dependencies: &[Dependency],
    dotted: bool,
    changes: &mut Vec<(Utf8PathBuf, String, String)>,
) -> anyhow::Result<()> {
    let member_content = read_to_string(path)?;
    let mut member_doc: DocumentMut = member_content.parse()?;

    for dep in dependencies {
        let memmber_dependencies = match dep.kind {
            DependencyKind::Normal => member_doc["dependencies"].as_table_mut(),
            DependencyKind::Development => member_doc["dev-dependencies"].as_table_mut(),
            DependencyKind::Build => member_doc["build-dependencies"].as_table_mut(),
            _ => None,
        };

        if let Some(member_dependencies) = memmber_dependencies {
            update_dependency(member_dependencies, dep, dotted);
        }
    }

    changes.push((path.to_path_buf(), member_content, member_doc.to_string()));

    Ok(())
}

fn update_dependency(member_dependencies: &mut Table, dep: &Dependency, dotted: bool) {
    if let Some(entry) = member_dependencies[&dep.name].as_table_like_mut() {
        entry.remove("version");
        entry.remove("default-features");
        entry.insert("workspace", value(true));
        entry.fmt();
    } else {
        let mut entry = InlineTable::new();
        entry.set_dotted(dotted);
        entry.insert("workspace", true.into());
        member_dependencies[&dep.name] = entry.into();
    }
}
