use anyhow::Result;
use toml::Value;

use crate::model::{DependencyRef, Ecosystem};

pub fn parse_pyproject_toml(content: &str) -> Result<Vec<DependencyRef>> {
    let data: Value = toml::from_str(content)?;
    let mut names = Vec::new();

    if let Some(project) = data.get("project").and_then(Value::as_table) {
        if let Some(deps) = project.get("dependencies").and_then(Value::as_array) {
            push_requirement_list(deps, &mut names);
        }

        if let Some(optional) = project
            .get("optional-dependencies")
            .and_then(Value::as_table)
        {
            for specs in optional.values() {
                if let Some(items) = specs.as_array() {
                    push_requirement_list(items, &mut names);
                }
            }
        }
    }

    if let Some(groups) = data.get("dependency-groups").and_then(Value::as_table) {
        for specs in groups.values() {
            if let Some(items) = specs.as_array() {
                push_requirement_list(items, &mut names);
            }
        }
    }

    if let Some(poetry) = data
        .get("tool")
        .and_then(Value::as_table)
        .and_then(|tool| tool.get("poetry"))
        .and_then(Value::as_table)
    {
        if let Some(deps) = poetry.get("dependencies").and_then(Value::as_table) {
            for dep_name in deps.keys() {
                if dep_name.eq_ignore_ascii_case("python") {
                    continue;
                }
                names.push(normalize_name(dep_name));
            }
        }

        if let Some(groups) = poetry.get("group").and_then(Value::as_table) {
            for group in groups.values() {
                let Some(group_table) = group.as_table() else {
                    continue;
                };
                let Some(deps) = group_table.get("dependencies").and_then(Value::as_table) else {
                    continue;
                };
                for dep_name in deps.keys() {
                    names.push(normalize_name(dep_name));
                }
            }
        }
    }

    Ok(to_dependency_refs(names))
}

pub fn parse_requirements_txt(content: &str) -> Result<Vec<DependencyRef>> {
    let mut names = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }
        if let Some(name) = parse_requirement_name(trimmed) {
            names.push(name);
        }
    }
    Ok(to_dependency_refs(names))
}

fn push_requirement_list(items: &[Value], names: &mut Vec<String>) {
    for item in items {
        let Some(spec) = item.as_str() else {
            continue;
        };
        if let Some(name) = parse_requirement_name(spec) {
            names.push(name);
        }
    }
}

fn to_dependency_refs(mut names: Vec<String>) -> Vec<DependencyRef> {
    names.sort();
    names.dedup();
    names
        .into_iter()
        .map(|name| DependencyRef {
            name,
            ecosystem: Ecosystem::Python,
            source_hint: None,
        })
        .collect()
}

fn parse_requirement_name(spec: &str) -> Option<String> {
    let trimmed = spec.trim_start();
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }

    let mut raw = String::new();
    raw.push(first);

    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            raw.push(ch);
            continue;
        }
        break;
    }
    Some(normalize_name(&raw))
}

fn normalize_name(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in name.chars() {
        let mapped = if ch == '_' || ch == '.' || ch == '-' {
            '-'
        } else {
            ch.to_ascii_lowercase()
        };
        if mapped == '-' {
            if prev_dash {
                continue;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
        }
        out.push(mapped);
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::parse_requirement_name;

    #[test]
    fn requirement_name_parsing() {
        assert_eq!(
            parse_requirement_name("requests>=2.32"),
            Some("requests".into())
        );
        assert_eq!(
            parse_requirement_name("pydantic[email]>=2"),
            Some("pydantic".into())
        );
        assert_eq!(
            parse_requirement_name("  my_pkg ; python_version<'3.13'"),
            Some("my-pkg".into())
        );
    }
}
