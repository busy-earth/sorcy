use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sorcy::resolve::RegistryConfig;

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
dependencies = ["requests>=2.31"]
"#,
    )
    .expect("write pyproject");

    let (base_url, hits, handle) = start_mock_server(
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

    let records = sorcy::run_with_config(project_root, config).expect("run scan");
    handle.join().expect("mock server thread");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].dependency, "requests");
    assert_eq!(records[0].source_url, "https://github.com/psf/requests");
    assert!(hits
        .lock()
        .expect("hits lock")
        .contains(&"/pypi/requests/json".to_string()));
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
    "left-pad": "^1.3.0"
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
                "/crates/serde",
                r#"{
                    "crate": {
                        "repository": "https://github.com/serde-rs/serde"
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

    let mut records = sorcy::run_with_config(project_root, config).expect("run scan");
    records.sort();
    handle.join().expect("mock server thread");

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].dependency, "left-pad");
    assert_eq!(
        records[0].source_url,
        "https://github.com/stevemao/left-pad"
    );
    assert_eq!(records[1].dependency, "serde");
    assert_eq!(records[1].source_url, "https://github.com/serde-rs/serde");
    let hits = hits.lock().expect("hits lock");
    assert!(hits.contains(&"/npm/left-pad".to_string()));
    assert!(hits.contains(&"/crates/serde".to_string()));
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
