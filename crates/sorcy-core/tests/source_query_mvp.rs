use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sorcy_core::model::{DependencyRef, Ecosystem, ManagedRepoStatus};
use sorcy_core::repo::{GitRunner, RepoManager, RepoManagerConfig, RepoUpdateStrategy};
use sorcy_core::resolve::SourceResolver;
use sorcy_core::FindFilesQuery;
use sorcy_core::{
    find_files, get_local_repo_for_dependency, get_local_repo_for_dependency_in_ecosystem,
    list_materialized_repos, read_repo_file,
};

#[test]
fn list_and_lookup_materialized_repos_from_project_materialization() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path();
    fs::write(
        project_root.join("pyproject.toml"),
        r#"
[project]
name = "demo"
version = "0.1.0"
dependencies = ["requests>=2.31", "missinglib>=0.1"]
"#,
    )
    .expect("write pyproject");

    let cache_dir = temp.path().join("cache");
    let (runner, _state) = FakeGitRunner::new();
    let repo_manager = RepoManager::with_git_runner(
        RepoManagerConfig {
            cache_dir: cache_dir.clone(),
            update_strategy: RepoUpdateStrategy::MissingOnly,
        },
        Box::new(runner),
    );
    let resolver = SourceQueryTestResolver;
    let materialization =
        sorcy_core::materialize_project_with_resolver(project_root, &resolver, &repo_manager)
            .expect("materialize project");

    let list = list_materialized_repos(&materialization);
    assert_eq!(list.len(), 2);
    let missing = list
        .iter()
        .find(|item| item.dependency_name == "missinglib")
        .expect("missing entry");
    assert_eq!(missing.ecosystem, Ecosystem::Python);
    assert_eq!(missing.local_path, None);
    assert_eq!(missing.status, None);
    assert!(!missing.is_materialized);

    let requests = list
        .iter()
        .find(|item| item.dependency_name == "requests")
        .expect("requests entry");
    assert_eq!(requests.ecosystem, Ecosystem::Python);
    assert_eq!(
        requests.local_path,
        Some(cache_dir.join("repos/github.com/psf/requests"))
    );
    assert_eq!(requests.status, Some(ManagedRepoStatus::Cloned));
    assert!(requests.is_materialized);

    let local_repo = get_local_repo_for_dependency(&materialization, "requests")
        .expect("local repo path for requests");
    assert_eq!(local_repo, cache_dir.join("repos/github.com/psf/requests"));
    assert!(get_local_repo_for_dependency(&materialization, "missinglib").is_none());
}

#[test]
fn ecosystem_aware_lookup_disambiguates_same_dependency_name() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path();
    fs::write(
        project_root.join("Cargo.toml"),
        r#"
[package]
name = "demo-rust"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
"#,
    )
    .expect("write Cargo.toml");
    fs::write(
        project_root.join("package.json"),
        r#"{
  "name": "demo-node",
  "version": "1.0.0",
  "dependencies": {
    "serde": "^1.0.0"
  }
}"#,
    )
    .expect("write package.json");

    let cache_dir = temp.path().join("cache");
    let (runner, _state) = FakeGitRunner::new();
    let repo_manager = RepoManager::with_git_runner(
        RepoManagerConfig {
            cache_dir: cache_dir.clone(),
            update_strategy: RepoUpdateStrategy::MissingOnly,
        },
        Box::new(runner),
    );
    let resolver = EcosystemAwareResolver;
    let materialization =
        sorcy_core::materialize_project_with_resolver(project_root, &resolver, &repo_manager)
            .expect("materialize project");

    let npm_repo =
        get_local_repo_for_dependency_in_ecosystem(&materialization, "serde", Ecosystem::Npm)
            .expect("npm serde path");
    let cargo_repo =
        get_local_repo_for_dependency_in_ecosystem(&materialization, "serde", Ecosystem::Cargo)
            .expect("cargo serde path");

    assert_eq!(npm_repo, cache_dir.join("repos/github.com/npm/serde"));
    assert_eq!(
        cargo_repo,
        cache_dir.join("repos/github.com/serde-rs/serde")
    );
    assert_ne!(npm_repo, cargo_repo);

    let default_lookup =
        get_local_repo_for_dependency(&materialization, "serde").expect("default serde path");
    assert_eq!(default_lookup, npm_repo);
}

#[test]
fn read_repo_file_rejects_unsafe_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let repo_root = temp.path().join("repo");
    fs::create_dir_all(repo_root.join("src")).expect("create src dir");
    fs::write(repo_root.join("src/lib.rs"), "pub fn demo() {}\n").expect("write source file");
    fs::write(temp.path().join("outside.txt"), "secret\n").expect("write outside file");

    let content =
        read_repo_file(&repo_root, PathBuf::from("src/lib.rs").as_path()).expect("read file");
    assert_eq!(content, "pub fn demo() {}\n");

    let err = read_repo_file(&repo_root, PathBuf::from("../outside.txt").as_path())
        .expect_err("reject path traversal");
    assert!(err.to_string().contains("not safe"));
}

#[test]
fn find_files_is_deterministic_and_supports_filters() {
    let temp = tempfile::tempdir().expect("temp dir");
    let repo_root = temp.path().join("repo");
    fs::create_dir_all(repo_root.join("src/bin")).expect("create nested dirs");
    fs::create_dir_all(repo_root.join("docs")).expect("create docs");
    fs::write(repo_root.join("Cargo.toml"), "[package]\nname=\"demo\"\n").expect("write cargo");
    fs::write(repo_root.join("src/lib.rs"), "pub fn lib() {}\n").expect("write lib");
    fs::write(repo_root.join("src/bin/tool.rs"), "fn main() {}\n").expect("write tool");
    fs::write(repo_root.join("docs/readme.md"), "# docs\n").expect("write docs");

    let query = FindFilesQuery {
        glob_pattern: Some("src/*.rs".to_string()),
        path_contains: Some("src".to_string()),
        extension: Some("rs".to_string()),
        max_results: None,
    };

    let first = find_files(&repo_root, &query).expect("first query");
    let second = find_files(&repo_root, &query).expect("second query");
    assert_eq!(first, second);
    assert_eq!(
        first,
        vec![
            PathBuf::from("src/bin/tool.rs"),
            PathBuf::from("src/lib.rs"),
        ]
    );

    let limited = find_files(
        &repo_root,
        &FindFilesQuery {
            max_results: Some(1),
            ..query
        },
    )
    .expect("limited query");
    assert_eq!(limited.len(), 1);
}

struct SourceQueryTestResolver;

impl SourceResolver for SourceQueryTestResolver {
    fn resolve(&self, dep: &DependencyRef) -> Option<String> {
        match dep.name.as_str() {
            "requests" => Some("https://github.com/psf/requests".to_string()),
            _ => None,
        }
    }
}

struct EcosystemAwareResolver;

impl SourceResolver for EcosystemAwareResolver {
    fn resolve(&self, dep: &DependencyRef) -> Option<String> {
        match (dep.ecosystem.clone(), dep.name.as_str()) {
            (Ecosystem::Npm, "serde") => Some("https://github.com/npm/serde".to_string()),
            (Ecosystem::Cargo, "serde") => Some("https://github.com/serde-rs/serde".to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
struct FakeGitState;

#[derive(Clone)]
struct FakeGitRunner {
    state: Arc<Mutex<FakeGitState>>,
}

impl FakeGitRunner {
    fn new() -> (Self, Arc<Mutex<FakeGitState>>) {
        let state = Arc::new(Mutex::new(FakeGitState));
        (
            Self {
                state: Arc::clone(&state),
            },
            state,
        )
    }
}

impl GitRunner for FakeGitRunner {
    fn run(&self, args: &[String]) -> Result<(), String> {
        if args.first().map(String::as_str) == Some("clone") {
            if args.len() != 3 {
                return Err(format!("unexpected clone args: {args:?}"));
            }
            let target = PathBuf::from(&args[2]);
            drop(self.state.lock().map_err(|_| "lock error".to_string())?);
            fs::create_dir_all(target.join(".git"))
                .map_err(|err| format!("failed creating fake clone target: {err}"))?;
            return Ok(());
        }
        Err(format!("unexpected git args: {args:?}"))
    }
}
