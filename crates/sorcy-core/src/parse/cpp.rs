use anyhow::Result;
use regex::Regex;
use serde_json::Value;

use crate::model::{DependencyRef, Ecosystem, ManifestKind};

use super::ManifestParser;

pub struct CppParser;

impl ManifestParser for CppParser {
    fn supports(&self, kind: ManifestKind) -> bool {
        matches!(
            kind,
            ManifestKind::VcpkgJson
                | ManifestKind::VcpkgConfigurationJson
                | ManifestKind::ConanfileTxt
                | ManifestKind::ConanfilePy
        )
    }

    fn parse(&self, kind: ManifestKind, content: &str) -> Result<Vec<DependencyRef>> {
        match kind {
            ManifestKind::VcpkgJson => parse_vcpkg_json(content),
            ManifestKind::VcpkgConfigurationJson => parse_vcpkg_configuration_json(content),
            ManifestKind::ConanfileTxt => parse_conanfile_txt(content),
            ManifestKind::ConanfilePy => parse_conanfile_py(content),
            _ => Ok(Vec::new()),
        }
    }
}

pub fn parse_vcpkg_json(content: &str) -> Result<Vec<DependencyRef>> {
    let data: Value = serde_json::from_str(content)?;
    let mut output = Vec::new();

    let Some(deps) = data.get("dependencies").and_then(Value::as_array) else {
        return Ok(output);
    };

    for entry in deps {
        match entry {
            Value::String(name) => push_dep(name, None, &mut output),
            Value::Object(obj) => {
                let Some(name) = obj.get("name").and_then(Value::as_str) else {
                    continue;
                };
                push_dep(name, None, &mut output);
            }
            _ => {}
        }
    }

    dedup(output)
}

pub fn parse_vcpkg_configuration_json(content: &str) -> Result<Vec<DependencyRef>> {
    let data: Value = serde_json::from_str(content)?;
    let mut output = Vec::new();

    if let Some(registries) = data.get("registries").and_then(Value::as_array) {
        for registry in registries {
            let Some(registry_obj) = registry.as_object() else {
                continue;
            };
            let Some(repository) = registry_obj.get("repository").and_then(Value::as_str) else {
                continue;
            };
            let Some(packages) = registry_obj.get("packages").and_then(Value::as_array) else {
                continue;
            };
            for package in packages {
                if let Some(name) = package.as_str() {
                    push_dep(name, Some(repository.to_string()), &mut output);
                }
            }
        }
    }

    dedup(output)
}

pub fn parse_conanfile_txt(content: &str) -> Result<Vec<DependencyRef>> {
    let mut output = Vec::new();
    let mut section = String::new();
    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }

        if section == "requires" || section == "tool_requires" {
            if let Some(name) = parse_conan_reference_name(line) {
                push_dep(name, None, &mut output);
            }
        }
    }

    dedup(output)
}

pub fn parse_conanfile_py(content: &str) -> Result<Vec<DependencyRef>> {
    let requires_call_re = Regex::new(r#"self\.requires\(\s*["']([^"']+)["']"#)?;
    let requires_block_re =
        Regex::new(r#"(?s)requires\s*=\s*(\([^)]+\)|\[[^\]]+\]|["'][^"']+["'])"#)?;
    let string_re = Regex::new(r#"["']([^"']+)["']"#)?;

    let mut output = Vec::new();

    for cap in requires_call_re.captures_iter(content) {
        if let Some(name) = parse_conan_reference_name(&cap[1]) {
            push_dep(name, None, &mut output);
        }
    }

    for cap in requires_block_re.captures_iter(content) {
        let block = &cap[1];
        for string_cap in string_re.captures_iter(block) {
            if let Some(name) = parse_conan_reference_name(&string_cap[1]) {
                push_dep(name, None, &mut output);
            }
        }
    }

    dedup(output)
}

fn parse_conan_reference_name(reference: &str) -> Option<&str> {
    let trimmed = reference.trim();
    if trimmed.is_empty() {
        return None;
    }
    let name = trimmed
        .split('/')
        .next()
        .unwrap_or(trimmed)
        .split('@')
        .next()
        .unwrap_or(trimmed)
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn push_dep(name: &str, source_hint: Option<String>, out: &mut Vec<DependencyRef>) {
    let normalized = name.trim();
    if normalized.is_empty() {
        return;
    }
    out.push(DependencyRef {
        name: normalized.to_string(),
        ecosystem: Ecosystem::Cpp,
        source_hint,
    });
}

fn dedup(mut items: Vec<DependencyRef>) -> Result<Vec<DependencyRef>> {
    items.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.source_hint.cmp(&b.source_hint))
    });
    items.dedup_by(|a, b| a.name == b.name && a.source_hint == b.source_hint);
    Ok(items)
}
