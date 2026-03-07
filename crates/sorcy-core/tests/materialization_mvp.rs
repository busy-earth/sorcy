use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sorcy_core::model::{DependencyRef, ManagedRepoStatus, ResolutionOrigin};
use sorcy_core::repo::{GitRunner, RepoManager, RepoManagerConfig, RepoUpdateStrategy};
use sorcy_core::resolve::SourceResolver;

#[test]
fn materialization_is_deterministic_idempotent_and_preserves_unresolved_entries() {
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
    fs::write(
        project_root.join("vcpkg-configuration.json"),
        r#"{
  "registries": [
    {
      "kind": "git",
      "repository": "https://github.com/fmtlib/fmt",
      "packages": ["fmt"]
    }
  ]
}"#,
    )
    .expect("write vcpkg config");

    let cache_dir = temp.path().join("cache");
    let (runner, state) = FakeGitRunner::new();
    let repo_manager = RepoManager::with_git_runner(
        RepoManagerConfig {
            cache_dir: cache_dir.clone(),
            update_strategy: RepoUpdateStrategy::MissingOnly,
        },
        Box::new(runner),
    );

    let resolver = MaterializeTestResolver;
    let first =
        sorcy_core::materialize_project_with_resolver(project_root, &resolver, &repo_manager)
            .expect("first materialization");

    let requests = first
        .materialized_resolutions
        .iter()
        .find(|x| x.resolution.dependency_name == "requests")
        .expect("requests resolution");
    let fmt = first
        .materialized_resolutions
        .iter()
        .find(|x| x.resolution.dependency_name == "fmt")
        .expect("fmt resolution");
    let missing = first
        .materialized_resolutions
        .iter()
        .find(|x| x.resolution.dependency_name == "missinglib")
        .expect("missing resolution");

    assert_eq!(
        requests
            .managed_repo
            .as_ref()
            .expect("managed requests")
            .local_path,
        cache_dir.join("repos/github.com/psf/requests")
    );
    assert_eq!(
        requests
            .managed_repo
            .as_ref()
            .expect("managed requests")
            .status,
        ManagedRepoStatus::Cloned
    );
    assert_eq!(
        fmt.resolution.resolution_origin,
        ResolutionOrigin::SourceHint
    );
    assert_eq!(
        fmt.managed_repo.as_ref().expect("managed fmt").local_path,
        cache_dir.join("repos/github.com/fmtlib/fmt")
    );
    assert_eq!(
        fmt.managed_repo.as_ref().expect("managed fmt").status,
        ManagedRepoStatus::Cloned
    );
    assert!(missing.managed_repo.is_none());
    assert_eq!(
        missing.resolution.resolution_origin,
        ResolutionOrigin::Unresolved
    );

    let clone_calls_after_first = state.lock().expect("fake state lock").clone_calls.len();
    assert_eq!(clone_calls_after_first, 2);
    assert!(repo_manager.metadata_path().exists());

    let persisted = repo_manager
        .load_managed_repos()
        .expect("load persisted metadata");
    assert_eq!(persisted.len(), 2);
    assert_eq!(
        persisted
            .iter()
            .map(|x| x.normalized_source_url.as_str())
            .collect::<Vec<_>>(),
        vec![
            "https://github.com/fmtlib/fmt",
            "https://github.com/psf/requests"
        ]
    );

    let second =
        sorcy_core::materialize_project_with_resolver(project_root, &resolver, &repo_manager)
            .expect("second materialization");
    let clone_calls_after_second = state.lock().expect("fake state lock").clone_calls.len();
    assert_eq!(clone_calls_after_second, 2);

    let second_statuses = second
        .materialized_resolutions
        .iter()
        .filter_map(|x| x.managed_repo.as_ref().map(|repo| repo.status))
        .collect::<Vec<_>>();
    assert_eq!(
        second_statuses,
        vec![ManagedRepoStatus::Unchanged, ManagedRepoStatus::Unchanged]
    );

    assert_eq!(
        first.materialized_resolutions.len(),
        second.materialized_resolutions.len()
    );
    let first_fingerprint = first
        .materialized_resolutions
        .iter()
        .map(|x| {
            (
                x.resolution.dependency_name.clone(),
                x.resolution
                    .source_repo
                    .as_ref()
                    .map(|repo| repo.normalized_source_url.clone()),
                x.managed_repo.as_ref().map(|repo| repo.local_path.clone()),
            )
        })
        .collect::<Vec<_>>();
    let second_fingerprint = second
        .materialized_resolutions
        .iter()
        .map(|x| {
            (
                x.resolution.dependency_name.clone(),
                x.resolution
                    .source_repo
                    .as_ref()
                    .map(|repo| repo.normalized_source_url.clone()),
                x.managed_repo.as_ref().map(|repo| repo.local_path.clone()),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(first_fingerprint, second_fingerprint);
}

#[test]
fn clone_failure_is_captured_without_failing_whole_materialization() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path();
    fs::write(
        project_root.join("pyproject.toml"),
        r#"
[project]
name = "demo"
version = "0.1.0"
dependencies = ["requests>=2.31", "failingdep>=1.0"]
"#,
    )
    .expect("write pyproject");

    let cache_dir = temp.path().join("cache");
    let (runner, state) = FakeGitRunner::new();
    state
        .lock()
        .expect("fake state lock")
        .fail_clone_urls
        .insert("https://github.com/example/failingdep".to_string());
    let repo_manager = RepoManager::with_git_runner(
        RepoManagerConfig {
            cache_dir,
            update_strategy: RepoUpdateStrategy::MissingOnly,
        },
        Box::new(runner),
    );

    let resolver = MaterializeTestResolver;
    let result =
        sorcy_core::materialize_project_with_resolver(project_root, &resolver, &repo_manager)
            .expect("materialization should not crash");

    let failed = result
        .materialized_resolutions
        .iter()
        .find(|x| x.resolution.dependency_name == "failingdep")
        .expect("failingdep resolution");
    assert_eq!(
        failed
            .managed_repo
            .as_ref()
            .expect("managed failing repo")
            .status,
        ManagedRepoStatus::Failed
    );
    assert!(failed
        .managed_repo
        .as_ref()
        .and_then(|repo| repo.error_message.as_ref())
        .is_some());

    let successful = result
        .materialized_resolutions
        .iter()
        .find(|x| x.resolution.dependency_name == "requests")
        .expect("requests resolution");
    assert_eq!(
        successful
            .managed_repo
            .as_ref()
            .expect("managed requests repo")
            .status,
        ManagedRepoStatus::Cloned
    );
}

#[test]
fn fetch_if_present_updates_existing_repo_without_recloning() {
    let temp = tempfile::tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let source_repo = sorcy_core::SourceRepo {
        normalized_source_url: "https://github.com/serde-rs/serde".to_string(),
        host: "github.com".to_string(),
        owner: "serde-rs".to_string(),
        repo: "serde".to_string(),
    };

    let (runner, state) = FakeGitRunner::new();
    let manager = RepoManager::with_git_runner(
        RepoManagerConfig {
            cache_dir: cache_dir.clone(),
            update_strategy: RepoUpdateStrategy::FetchIfPresent,
        },
        Box::new(runner),
    );
    let local_path = manager.local_repo_path(&source_repo);
    fs::create_dir_all(local_path.join(".git")).expect("seed fake existing repo");

    let result = manager.materialize(&source_repo);
    assert_eq!(result.status, ManagedRepoStatus::Updated);
    let snapshot = state.lock().expect("fake state lock");
    assert_eq!(snapshot.clone_calls.len(), 0);
    assert_eq!(snapshot.fetch_calls.len(), 1);
}

struct MaterializeTestResolver;

impl SourceResolver for MaterializeTestResolver {
    fn resolve(&self, dep: &DependencyRef) -> Option<String> {
        if let Some(hint) = dep.source_hint.as_ref() {
            return Some(hint.clone());
        }
        match dep.name.as_str() {
            "requests" => Some("https://github.com/psf/requests".to_string()),
            "failingdep" => Some("https://github.com/example/failingdep".to_string()),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
struct FakeGitState {
    clone_calls: Vec<(String, PathBuf)>,
    fetch_calls: Vec<PathBuf>,
    fail_clone_urls: BTreeSet<String>,
}

#[derive(Clone)]
struct FakeGitRunner {
    state: Arc<Mutex<FakeGitState>>,
}

impl FakeGitRunner {
    fn new() -> (Self, Arc<Mutex<FakeGitState>>) {
        let state = Arc::new(Mutex::new(FakeGitState::default()));
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
            let source_url = args[1].clone();
            let target = PathBuf::from(&args[2]);
            let mut state = self.state.lock().map_err(|_| "lock error".to_string())?;
            state.clone_calls.push((source_url.clone(), target.clone()));
            if state.fail_clone_urls.contains(&source_url) {
                return Err(format!("simulated clone failure for {source_url}"));
            }
            drop(state);
            fs::create_dir_all(target.join(".git"))
                .map_err(|err| format!("failed creating fake clone target: {err}"))?;
            return Ok(());
        }

        if args.first().map(String::as_str) == Some("-C")
            && args.get(2).map(String::as_str) == Some("fetch")
        {
            let target = PathBuf::from(
                args.get(1)
                    .ok_or_else(|| format!("unexpected fetch args: {args:?}"))?,
            );
            self.state
                .lock()
                .map_err(|_| "lock error".to_string())?
                .fetch_calls
                .push(target);
            return Ok(());
        }

        Err(format!("unexpected git args: {args:?}"))
    }
}
