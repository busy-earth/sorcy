use anyhow::Result;
use toml::Value;

use crate::model::{DependencyRef, Ecosystem};

const TOP_LEVEL_SECTIONS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

pub fn parse_cargo_toml(content: &str) -> Result<Vec<DependencyRef>> {
    let data: Value = toml::from_str(content)?;
    let mut names = Vec::new();

    if let Some(root) = data.as_table() {
        for section in TOP_LEVEL_SECTIONS {
            push_table_keys(root.get(*section), &mut names);
        }

        if let Some(workspace) = root.get("workspace").and_then(Value::as_table) {
            push_table_keys(workspace.get("dependencies"), &mut names);
        }

        if let Some(targets) = root.get("target").and_then(Value::as_table) {
            for target_spec in targets.values() {
                let Some(target_table) = target_spec.as_table() else {
                    continue;
                };
                for section in TOP_LEVEL_SECTIONS {
                    push_table_keys(target_table.get(*section), &mut names);
                }
            }
        }
    }

    names.sort();
    names.dedup();
    Ok(names
        .into_iter()
        .map(|name| DependencyRef {
            name,
            ecosystem: Ecosystem::Cargo,
            source_hint: None,
        })
        .collect())
}

fn push_table_keys(value: Option<&Value>, out: &mut Vec<String>) {
    let Some(table) = value.and_then(Value::as_table) else {
        return;
    };
    for key in table.keys() {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        out.push(key.to_string());
    }
}
