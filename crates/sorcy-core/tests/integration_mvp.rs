use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sorcy_core::model::{DependencyRef, ResolutionOrigin};
use sorcy_core::resolve::{RegistryConfig, SourceResolver};

#[test]
fn mvp_loop_python_repo_resolves_source_url() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path();

    fs::write(
        project_root.join("pyproject.toml"),
        r#"
[project]
name = "demo"
version = "0.1.0"
dependencies = ["requests>=2.31", "flask==3.0.0"]
"#,
    )
    .expect("write pyproject");

    let (base_url, hits, handle) = start_mock_server(
        HashMap::from([
            (
                "/pypi/requests/json",
                r#"{
                    "info": {
                        "project_urls": {
                            "Source": "https://github.com/psf/requests"
                        }
                    }
                }"#,
            ),
            (
                "/pypi/flask/json",
                r#"{
                    "info": {
                        "project_urls": {
                            "Repository": "https://github.com/pallets/flask"
                        }
                    }
                }"#,
            ),
        ]),
        2,
    );

    let config = RegistryConfig {
        pypi_base_url: format!("{base_url}/pypi"),
        npm_base_url: format!("{base_url}/npm"),
        crates_base_url: format!("{base_url}/crates"),
        ..RegistryConfig::default()
    };

    let records = sorcy_core::run_with_config(project_root, config).expect("run scan");
    handle.join().expect("mock server thread");

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].dependency, "flask");
    assert_eq!(records[0].source_url, "https://github.com/pallets/flask");
    assert_eq!(records[1].dependency, "requests");
    assert_eq!(records[1].source_url, "https://github.com/psf/requests");
    assert!(hits
        .lock()
        .expect("hits lock")
        .contains(&"/pypi/requests/json".to_string()));
    assert!(hits
        .lock()
        .expect("hits lock")
        .contains(&"/pypi/flask/json".to_string()));
}

#[test]
fn resolves_npm_and_cargo_from_registry_metadata() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path();

    fs::write(
        project_root.join("package.json"),
        r#"{
  "name": "demo-node",
  "version": "1.0.0",
  "dependencies": {
    "left-pad": "^1.3.0",
    "lodash": "^4.17.21"
  }
}"#,
    )
    .expect("write package.json");

    fs::write(
        project_root.join("Cargo.toml"),
        r#"
[package]
name = "demo-rust"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
tokio = "1"
"#,
    )
    .expect("write Cargo.toml");

    let (base_url, hits, handle) = start_mock_server(
        HashMap::from([
            (
                "/npm/left-pad",
                r#"{
                    "repository": {
                        "type": "git",
                        "url": "git+https://github.com/stevemao/left-pad.git"
                    }
                }"#,
            ),
            (
                "/npm/lodash",
                r#"{
                    "homepage": "https://github.com/lodash/lodash"
                }"#,
            ),
            (
                "/crates/serde",
                r#"{
                    "crate": {
                        "repository": "https://github.com/serde-rs/serde"
                    }
                }"#,
            ),
            (
                "/crates/tokio",
                r#"{
                    "crate": {
                        "homepage": "https://github.com/tokio-rs/tokio"
                    }
                }"#,
            ),
        ]),
        4,
    );

    let config = RegistryConfig {
        pypi_base_url: format!("{base_url}/pypi"),
        npm_base_url: format!("{base_url}/npm"),
        crates_base_url: format!("{base_url}/crates"),
        ..RegistryConfig::default()
    };

    let mut records = sorcy_core::run_with_config(project_root, config).expect("run scan");
    records.sort();
    handle.join().expect("mock server thread");

    assert_eq!(records.len(), 4);
    assert_eq!(records[0].dependency, "left-pad");
    assert_eq!(
        records[0].source_url,
        "https://github.com/stevemao/left-pad"
    );
    assert_eq!(records[1].dependency, "lodash");
    assert_eq!(records[1].source_url, "https://github.com/lodash/lodash");
    assert_eq!(records[2].dependency, "serde");
    assert_eq!(records[2].source_url, "https://github.com/serde-rs/serde");
    assert_eq!(records[3].dependency, "tokio");
    assert_eq!(records[3].source_url, "https://github.com/tokio-rs/tokio");
    let hits = hits.lock().expect("hits lock");
    assert!(hits.contains(&"/npm/left-pad".to_string()));
    assert!(hits.contains(&"/npm/lodash".to_string()));
    assert!(hits.contains(&"/crates/serde".to_string()));
    assert!(hits.contains(&"/crates/tokio".to_string()));
}

#[test]
fn resolves_cpp_from_vcpkg_configuration_repository_hints() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path();

    fs::write(
        project_root.join("vcpkg-configuration.json"),
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
    )
    .expect("write vcpkg-configuration.json");

    let mut records =
        sorcy_core::run_with_config(project_root, RegistryConfig::default()).expect("run scan");
    records.sort();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].dependency, "fmt");
    assert_eq!(records[0].source_url, "https://github.com/fmtlib/fmt");
    assert_eq!(records[1].dependency, "spdlog");
    assert_eq!(records[1].source_url, "https://github.com/gabime/spdlog");
}

#[test]
fn project_scan_preserves_provenance_and_legacy_output_is_derived() {
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
        project_root.join("requirements-dev.txt"),
        "requests==2.31.0\n",
    )
    .expect("write requirements");
    fs::write(
        project_root.join("vcpkg-configuration.json"),
        r#"{
  "registries": [
    {
      "kind": "git",
      "repository": "git@github.com:fmtlib/fmt.git",
      "packages": ["fmt"]
    }
  ]
}"#,
    )
    .expect("write vcpkg config");

    let resolver = StaticTestResolver;
    let scan =
        sorcy_core::scan_project_with_resolver(project_root, &resolver).expect("project scan");

    assert_eq!(scan.root_path, project_root.to_path_buf());
    assert_eq!(scan.manifests.len(), 3);
    assert_eq!(scan.dependencies.len(), 4);
    assert_eq!(scan.resolutions.len(), 4);

    let fmt_dep = scan
        .dependencies
        .iter()
        .find(|x| x.dependency_name == "fmt")
        .expect("fmt dependency record");
    assert!(fmt_dep
        .manifest_path
        .to_string_lossy()
        .ends_with("vcpkg-configuration.json"));
    assert!(fmt_dep.source_hint.is_some());

    let fmt_resolution = scan
        .resolutions
        .iter()
        .find(|x| x.dependency_name == "fmt")
        .expect("fmt resolution");
    assert_eq!(
        fmt_resolution.resolution_origin,
        ResolutionOrigin::SourceHint
    );
    assert_eq!(
        fmt_resolution
            .source_repo
            .as_ref()
            .expect("source repo")
            .normalized_source_url,
        "https://github.com/fmtlib/fmt"
    );

    let requests_resolution = scan
        .resolutions
        .iter()
        .find(|x| {
            x.dependency_name == "requests"
                && x.resolution_origin == ResolutionOrigin::RegistryMetadata
        })
        .expect("requests resolution");
    assert_eq!(
        requests_resolution
            .source_repo
            .as_ref()
            .expect("source repo")
            .normalized_source_url,
        "https://github.com/psf/requests"
    );

    let unresolved = scan
        .resolutions
        .iter()
        .find(|x| x.dependency_name == "missinglib")
        .expect("missinglib resolution");
    assert_eq!(unresolved.resolution_origin, ResolutionOrigin::Unresolved);
    assert!(unresolved.source_repo.is_none());

    let records = sorcy_core::run_with_resolver(project_root, &resolver).expect("compat output");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].dependency, "fmt");
    assert_eq!(records[1].dependency, "requests");
    assert_eq!(records[0].source_url, "https://github.com/fmtlib/fmt");
    assert_eq!(records[1].source_url, "https://github.com/psf/requests");
}

#[test]
fn registry_metadata_origin_is_set_for_registry_resolved_dependency() {
    let temp = tempfile::tempdir().expect("temp dir");
    let project_root = temp.path();

    fs::write(
        project_root.join("pyproject.toml"),
        r#"
[project]
name = "demo"
version = "0.1.0"
dependencies = ["requests>=2.31"]
"#,
    )
    .expect("write pyproject");

    let (base_url, _hits, handle) = start_mock_server(
        HashMap::from([(
            "/pypi/requests/json",
            r#"{
                "info": {
                    "project_urls": {
                        "Source": "https://github.com/psf/requests"
                    }
                }
            }"#,
        )]),
        1,
    );

    let config = RegistryConfig {
        pypi_base_url: format!("{base_url}/pypi"),
        npm_base_url: format!("{base_url}/npm"),
        crates_base_url: format!("{base_url}/crates"),
        ..RegistryConfig::default()
    };

    let scan = sorcy_core::scan_project_with_config(project_root, config).expect("scan project");
    handle.join().expect("server thread");

    assert_eq!(scan.resolutions.len(), 1);
    assert_eq!(scan.resolutions[0].dependency_name, "requests");
    assert_eq!(
        scan.resolutions[0].resolution_origin,
        ResolutionOrigin::RegistryMetadata
    );
}

struct StaticTestResolver;

impl SourceResolver for StaticTestResolver {
    fn resolve(&self, dep: &DependencyRef) -> Option<String> {
        if dep.source_hint.is_some() && dep.name == "fmt" {
            return Some("https://github.com/fmtlib/fmt".to_string());
        }
        match dep.name.as_str() {
            "requests" => Some("https://github.com/psf/requests".to_string()),
            _ => None,
        }
    }
}

fn start_mock_server(
    routes: HashMap<&'static str, &'static str>,
    expected_requests: usize,
) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind tcp listener");
    listener
        .set_nonblocking(true)
        .expect("set listener non-blocking");
    let addr = listener.local_addr().expect("local addr");

    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_for_thread = Arc::clone(&hits);

    let handle = thread::spawn(move || {
        let mut served = 0usize;
        let started = Instant::now();

        while served < expected_requests {
            if started.elapsed() > Duration::from_secs(8) {
                panic!("mock server timeout waiting for requests");
            }

            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    let mut buffer = [0_u8; 4096];
                    let bytes = stream.read(&mut buffer).expect("read request");
                    let request = String::from_utf8_lossy(&buffer[..bytes]);
                    let first_line = request.lines().next().unwrap_or_default();
                    let path = first_line
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("/")
                        .to_string();
                    hits_for_thread
                        .lock()
                        .expect("hits lock")
                        .push(path.clone());

                    let body = routes.get(path.as_str()).copied().unwrap_or(r#"{}"#);
                    let status = if routes.contains_key(path.as_str()) {
                        "200 OK"
                    } else {
                        "404 Not Found"
                    };
                    let response = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream
                        .write_all(response.as_bytes())
                        .expect("write response");
                    served += 1;
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("mock server accept error: {err}"),
            }
        }
    });

    (format!("http://{addr}"), hits, handle)
}
