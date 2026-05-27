use cargo_metadata::{
    Dependency, DependencyKind, Metadata,
    camino::{Utf8Path, Utf8PathBuf},
    semver::VersionReq,
};
use patcher::{DiffAlgorithm, Differ, MultifilePatch};
use pathdiff::diff_utf8_paths;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::read_to_string;
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, table, value};

use crate::MemberDependency;

/// Render a `VersionReq` using Cargo's preferred syntax: drop a leading
/// caret for single-comparator caret reqs (e.g. `^1.2` -> `1.2`).
fn format_req(req: &VersionReq) -> String {
    let s = req.to_string();
    if let Some(rest) = s.strip_prefix('^')
        && !rest.contains([',', ' ', '<', '>', '=', '~', '^', '*'])
    {
        rest.to_string()
    } else {
        s
    }
}

/// Score a `VersionReq` by its first comparator's (major, minor, patch)
/// so divergent member reqs can be reconciled by picking the highest
/// minimum. Complex multi-comparator reqs fall back to (0,0,0); the
/// existing iteration order then picks the first such entry.
fn req_floor(req: &VersionReq) -> (u64, u64, u64) {
    req.comparators
        .first()
        .map(|c| (c.major, c.minor.unwrap_or(0), c.patch.unwrap_or(0)))
        .unwrap_or((0, 0, 0))
}

#[derive(Default)]
struct MemberChanges {
    to_workspace: Vec<Dependency>,
    to_inline: Vec<(String, Item, DependencyKind)>,
}

pub fn generate_diff(
    add: &BTreeMap<String, Vec<MemberDependency>>,
    remove: &BTreeSet<String>,
    inline: &BTreeMap<String, MemberDependency>,
    metadata: &Metadata,
    dotted: bool,
) -> anyhow::Result<()> {
    let mut changes = Vec::with_capacity(add.len() + inline.len() + 1);

    let workspace_path = metadata.workspace_root.join("Cargo.toml");
    let workspace_content = read_to_string(&workspace_path)?;
    let mut workspace_doc: DocumentMut = workspace_content.parse()?;

    // Capture workspace dep items before removing them (needed for inline)
    let mut inline_items: BTreeMap<String, Item> = BTreeMap::new();
    if let Some(workspace_dependencies) = workspace_doc
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table_like())
    {
        for name in inline.keys() {
            if let Some(item) = workspace_dependencies.get(name) {
                inline_items.insert(name.clone(), item.clone());
            }
        }
    }

    if let Some(workspace_dependencies) = workspace_doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("dependencies"))
        .and_then(|d| d.as_table_like_mut())
    {
        for name in remove {
            workspace_dependencies.remove(name);
        }
    }

    let mut member_changes: BTreeMap<Utf8PathBuf, MemberChanges> = BTreeMap::new();

    if !add.is_empty() {
        let Some(workspace_table) = workspace_doc
            .as_table_mut()
            .entry("workspace")
            .or_insert_with(table)
            .as_table_mut()
        else {
            anyhow::bail!("Invalid [workspace] entry");
        };
        // `Table::entry(...).or_insert(table())` does not reliably promote a
        // missing key into a real `Item::Table` for top-level workspace
        // sub-tables, so insert explicitly when missing.
        if !workspace_table.contains_key("dependencies") {
            workspace_table.insert("dependencies", table());
        }
        let Some(workspace_dependencies) = workspace_table
            .get_mut("dependencies")
            .and_then(|d| d.as_table_mut())
        else {
            anyhow::bail!("Invalid workspace dependencies entry");
        };

        for (name, members) in add {
            // Reconcile divergent member version reqs by picking the one
            // with the highest minimum version. This avoids silently
            // downgrading a member that pinned a newer floor.
            let mut dependency: Option<&Dependency> = None;
            let mut no_default_features = false;
            let mut features = BTreeSet::new();
            for member in members {
                match dependency {
                    None => dependency = Some(&member.dependency),
                    Some(current)
                        if req_floor(&member.dependency.req) > req_floor(&current.req) =>
                    {
                        dependency = Some(&member.dependency);
                    }
                    _ => {}
                }

                features.extend(member.dependency.features.iter().cloned());
                no_default_features |= !member.dependency.uses_default_features;

                member_changes
                    .entry(member.manifest_path.clone())
                    .or_default()
                    .to_workspace
                    .push(member.dependency.clone());
            }

            if let Some(dependency) = dependency {
                let req_str = format_req(&dependency.req);
                let value = if no_default_features || !features.is_empty() {
                    let mut entry = InlineTable::new();
                    entry.insert("version", req_str.into());

                    if no_default_features {
                        entry.insert("default-features", false.into());
                    }

                    if !features.is_empty() {
                        entry.insert("features", Array::from_iter(features).into());
                    }

                    entry.into()
                } else {
                    value(req_str)
                };

                // The dep may already exist in [workspace.dependencies] when
                // `add[name]` only contains inline holdouts being consolidated
                // onto an existing entry; preserve the existing entry (and its
                // features/default-features/etc.) in that case.
                if !workspace_dependencies.contains_key(name) {
                    workspace_dependencies.insert(name, value);
                }
            }
        }

        // Keep the workspace dependency table alphabetically sorted so newly
        // inserted entries land in their proper place rather than appended.
        workspace_dependencies.sort_values();
    }

    for (name, member) in inline {
        if let Some(item) = inline_items.remove(name) {
            member_changes
                .entry(member.manifest_path.clone())
                .or_default()
                .to_inline
                .push((name.clone(), item, member.dependency.kind));
        }
    }

    for (path, mc) in member_changes {
        update_member(&path, &mc, dotted, &metadata.workspace_root, &mut changes)?;
    }

    changes.push((workspace_path, workspace_content, workspace_doc.to_string()));

    let mut patches = Vec::new();

    for (path, original, modified) in changes {
        if original == modified {
            continue;
        }
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
    mc: &MemberChanges,
    dotted: bool,
    workspace_root: &Utf8Path,
    changes: &mut Vec<(Utf8PathBuf, String, String)>,
) -> anyhow::Result<()> {
    let member_content = read_to_string(path)?;
    let mut member_doc: DocumentMut = member_content.parse()?;
    let member_dir = path.parent().unwrap_or(path);

    for dep in &mc.to_workspace {
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

    for (name, item, kind) in &mc.to_inline {
        let memmber_dependencies = match kind {
            DependencyKind::Normal => member_doc["dependencies"].as_table_mut(),
            DependencyKind::Development => member_doc["dev-dependencies"].as_table_mut(),
            DependencyKind::Build => member_doc["build-dependencies"].as_table_mut(),
            _ => None,
        };

        if let Some(member_dependencies) = memmber_dependencies {
            inline_dependency(member_dependencies, name, item, workspace_root, member_dir);
        }
    }

    changes.push((path.to_path_buf(), member_content, member_doc.to_string()));

    Ok(())
}

fn update_dependency(member_dependencies: &mut Table, dep: &Dependency, dotted: bool) {
    if let Some(entry) = member_dependencies[&dep.name].as_table_like_mut() {
        entry.remove("version");
        entry.remove("default-features");
        // Collect remaining keys so we can re-insert them after `workspace = true`.
        let rest: Vec<(String, toml_edit::Value)> = entry
            .iter()
            .filter(|(k, _)| *k != "workspace")
            .filter_map(|(k, v)| v.as_value().map(|val| (k.to_string(), val.clone())))
            .collect();
        entry.clear();
        entry.insert("workspace", value(true));
        for (k, v) in rest {
            entry.insert(&k, Item::Value(v));
        }
        entry.fmt();
    } else {
        let mut entry = InlineTable::new();
        entry.set_dotted(dotted);
        entry.insert("workspace", true.into());
        member_dependencies[&dep.name] = entry.into();
    }
}

/// Rewrite a `path` value from workspace-root-relative to member-dir-relative.
/// If the path is absolute or the diff fails, return it unchanged.
fn rebase_path(ws_path: &str, workspace_root: &Utf8Path, member_dir: &Utf8Path) -> String {
    let abs = workspace_root.join(ws_path);
    diff_utf8_paths(&abs, member_dir)
        .map(|p| p.to_string())
        .unwrap_or_else(|| ws_path.to_string())
}

fn inline_dependency(
    member_dependencies: &mut Table,
    name: &str,
    ws_item: &Item,
    workspace_root: &Utf8Path,
    member_dir: &Utf8Path,
) {
    // Determine if the existing member entry has any extra keys besides `workspace`
    let mut extras: Vec<(String, Item)> = Vec::new();
    if let Some(entry) = member_dependencies
        .get(name)
        .and_then(|e| e.as_table_like())
    {
        for (k, v) in entry.iter() {
            if k != "workspace" {
                extras.push((k.to_string(), v.clone()));
            }
        }
    }

    if extras.is_empty() {
        // Replace member entry with workspace's item directly, rebasing any path.
        let rebased = rebase_ws_item(ws_item, workspace_root, member_dir);
        member_dependencies.insert(name, rebased);
    } else {
        // Merge workspace item fields with member's extras
        let mut merged = InlineTable::new();
        match ws_item {
            Item::Value(toml_edit::Value::String(s)) => {
                merged.insert("version", s.value().clone().into());
            }

            Item::Value(toml_edit::Value::InlineTable(t)) => {
                for (k, v) in t.iter() {
                    if k == "path"
                        && let Some(p) = v.as_str() {
                            merged.insert(k, rebase_path(p, workspace_root, member_dir).into());
                            continue;
                        }

                    merged.insert(k, v.clone());
                }
            }

            Item::Table(t) => {
                for (k, v) in t.iter() {
                    if k == "path"
                        && let Some(p) = v.as_str() {
                            merged.insert(k, rebase_path(p, workspace_root, member_dir).into());
                            continue;
                        }

                    if let Some(val) = v.as_value() {
                        merged.insert(k, val.clone());
                    }
                }
            }
            _ => {}
        }
        for (k, v) in extras {
            if let Some(val) = v.as_value() {
                merged.insert(&k, val.clone());
            }
        }
        merged.fmt();
        member_dependencies[name] = value(merged);
    }
}

/// Clone a workspace Item, rebasing any `path` value to be relative to `member_dir`.
fn rebase_ws_item(item: &Item, workspace_root: &Utf8Path, member_dir: &Utf8Path) -> Item {
    match item {
        Item::Value(toml_edit::Value::InlineTable(t)) => {
            let mut new_t = t.clone();
            if let Some(p) = t.get("path").and_then(|v| v.as_str()) {
                new_t.insert("path", rebase_path(p, workspace_root, member_dir).into());
            }

            Item::Value(toml_edit::Value::InlineTable(new_t))
        }

        Item::Table(t) => {
            let mut new_t = t.clone();
            if let Some(p) = t.get("path").and_then(|v| v.as_str()) {
                new_t.insert("path", value(rebase_path(p, workspace_root, member_dir)));
            }
            Item::Table(new_t)
        }

        // Plain version string — no path to rebase.
        other => other.clone(),
    }
}
