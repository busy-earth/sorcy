use std::collections::BTreeSet;
use std::fs;

use anyhow::Result;
use sorcy_core::repo::RepoUpdateStrategy;
use sorcy_core::resolve::RegistryConfig;

#[test]
#[ignore = "live network test; run manually with SORCY_LIVE_TESTS=1"]
fn live_registry_resolution_smoke_all_supported_ecosystems() -> Result<()> {
    require_live_opt_in();

    let temp = tempfile::tempdir()?;
    let root = temp.path();

    fs::write(
        root.join("pyproject.toml"),
        r#"
[project]
name = "live-smoke"
version = "0.1.0"
dependencies = ["requests>=2.31", "flask>=3.0.0"]
"#,
    )?;
    fs::write(
        root.join("package.json"),
        r#"{
  "name": "live-smoke",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0",
    "lodash": "^4.17.21"
  }
}"#,
    )?;
    fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "live-smoke"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
itoa = "1"
"#,
    )?;
    fs::write(
        root.join("vcpkg-configuration.json"),
        r#"{
  "registries": [
    {
      "kind": "git",
      "repository": "https://github.com/fmtlib/fmt",
      "packages": ["fmt"]
    },
    {
      "kind": "git",
      "repository": "git@github.com:gabime/spdlog.git",
      "packages": ["spdlog"]
    }
  ]
}"#,
    )?;

    let mut records = sorcy_core::run_with_config(
        root,
        RegistryConfig {
            http_timeout_seconds: 20,
            http_retries: 4,
            http_retry_backoff_ms: 200,
            ..RegistryConfig::default()
        },
    )?;
    records.sort();
    let got = records
        .into_iter()
        .map(|record| (record.dependency, record.source_url))
        .collect::<BTreeSet<_>>();

    assert!(got.contains(&(
        "requests".to_string(),
        "https://github.com/psf/requests".to_string()
    )));
    assert!(got.contains(&(
        "flask".to_string(),
        "https://github.com/pallets/flask".to_string()
    )));
    assert!(got.contains(&(
        "react".to_string(),
        "https://github.com/facebook/react".to_string()
    )));
    assert!(got.contains(&(
        "lodash".to_string(),
        "https://github.com/lodash/lodash".to_string()
    )));
    assert!(got.contains(&(
        "serde".to_string(),
        "https://github.com/serde-rs/serde".to_string()
    )));
    assert!(got.contains(&(
        "itoa".to_string(),
        "https://github.com/dtolnay/itoa".to_string()
    )));
    assert!(got.contains(&(
        "fmt".to_string(),
        "https://github.com/fmtlib/fmt".to_string()
    )));
    assert!(got.contains(&(
        "spdlog".to_string(),
        "https://github.com/gabime/spdlog".to_string()
    )));

    Ok(())
}

#[test]
#[ignore = "live network test; run manually with SORCY_LIVE_TESTS=1"]
fn live_materialization_smoke_two_real_repos() -> Result<()> {
    require_live_opt_in();

    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(
        root.join("package.json"),
        r#"{
  "name": "live-materialize",
  "version": "1.0.0",
  "dependencies": {
    "left-pad": "^1.3.0"
  }
}"#,
    )?;
    fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "live-materialize"
version = "0.1.0"
edition = "2021"

[dependencies]
itoa = "1"
"#,
    )?;

    let cache_dir = temp.path().join("cache");
    let config = sorcy_core::SorcyConfig {
        registry: RegistryConfig {
            http_timeout_seconds: 20,
            http_retries: 4,
            http_retry_backoff_ms: 200,
            ..RegistryConfig::default()
        },
        repo_cache_dir: cache_dir.clone(),
        repo_update_strategy: RepoUpdateStrategy::MissingOnly,
    };

    let materialization = sorcy_core::materialize_project_with_config(root, config)?;
    let materialized = materialization
        .materialized_resolutions
        .iter()
        .filter_map(|entry| entry.managed_repo.as_ref())
        .collect::<Vec<_>>();

    assert!(
        materialized.len() >= 2,
        "expected at least two materialized repositories"
    );
    assert!(materialized.iter().any(|repo| {
        repo.normalized_source_url == "https://github.com/stevemao/left-pad"
            && repo.local_path.exists()
    }));
    assert!(materialized.iter().any(|repo| {
        repo.normalized_source_url == "https://github.com/dtolnay/itoa" && repo.local_path.exists()
    }));

    Ok(())
}

fn require_live_opt_in() {
    let enabled = std::env::var("SORCY_LIVE_TESTS").unwrap_or_default();
    assert_eq!(
        enabled, "1",
        "live tests are opt-in; run with SORCY_LIVE_TESTS=1 and -- --ignored"
    );
}
