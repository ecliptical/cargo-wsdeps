use std::collections::{BTreeMap, BTreeSet};

use crate::MemberDependency;

pub fn print_changes(
    add: &BTreeMap<String, Vec<MemberDependency>>,
    remove: &BTreeSet<String>,
    inline: &BTreeMap<String, MemberDependency>,
) {
    if !add.is_empty() {
        println!("Move dependencies from individual crates to [workspace.dependencies]:");
        for (name, members) in add {
            println!(
                "  {name}: {}",
                members
                    .iter()
                    .map(|member| member.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    if !inline.is_empty() {
        println!(
            "Move single-use dependencies from [workspace.dependencies] back into the member:"
        );
        for (name, member) in inline {
            println!("  {name}: {}", member.name);
        }
    }

    if !remove.is_empty() {
        let remove_only: Vec<&String> =
            remove.iter().filter(|n| !inline.contains_key(*n)).collect();
        if !remove_only.is_empty() {
            println!("Remove dependencies from [workspace.dependencies]:");
            for name in remove_only {
                println!("  {name}");
            }
        }
    }
}
