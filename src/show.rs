use std::collections::{BTreeMap, BTreeSet};

use crate::MemberDependency;

pub fn print_changes(add: &BTreeMap<String, Vec<MemberDependency>>, remove: &BTreeSet<String>) {
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

    if !remove.is_empty() {
        println!("Remove dependencies from [workspace.dependencies]:");
        for name in remove {
            println!("  {name}");
        }
    }
}
