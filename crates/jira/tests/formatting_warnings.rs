use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[test]
#[cfg_attr(
    windows,
    ignore = "Windows debug CLI requires an increased stack reserve"
)]
fn cli_formatting_warnings_use_stderr_without_changing_json_or_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut requests = 0;
        while requests < 5 && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                thread::sleep(Duration::from_millis(10));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
            }
            let header_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .unwrap()
                + 4;
            let content_length = String::from_utf8_lossy(&request[..header_end])
                .lines()
                .find_map(|line| {
                    line.split_once(':').and_then(|(name, value)| {
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                })
                .unwrap_or(0);
            while request.len() < header_end + content_length {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                request.extend_from_slice(&buffer[..count]);
            }
            let headers = String::from_utf8_lossy(&request);
            let first_line = headers.lines().next().unwrap();
            let (status, body) = if first_line.starts_with("PUT ") {
                if first_line.contains("PROJ-FAIL") {
                    ("400 Bad Request", "")
                } else {
                    ("204 No Content", "")
                }
            } else {
                (
                    "200 OK",
                    r#"{"id":"1","key":"PROJ-1","fields":{"summary":"Example"}}"#,
                )
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            requests += 1;
        }
        assert_eq!(requests, 5);
    });

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp =
        std::env::temp_dir().join(format!("jirac-formatting-{}-{unique}", std::process::id()));
    std::fs::create_dir(&temp).unwrap();
    let description = temp.join("description.md");
    std::fs::write(&description, "[label](relative)").unwrap();

    for (version, key) in [("2", "PROJ-1"), ("3", "PROJ-1"), ("2", "PROJ-FAIL")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jirac"));
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("JIRA_") {
                command.env_remove(name);
            }
        }
        let output = command
            .args(["issue", "update", key, "--description-file"])
            .arg(&description)
            .arg("--json")
            .env("JIRA_URL", format!("http://{address}"))
            .env("JIRA_EMAIL", "test@example.invalid")
            .env("JIRA_TOKEN", "test")
            .env("JIRA_API_VERSION", version)
            .env("HOME", &temp)
            .env("USERPROFILE", &temp)
            .env("APPDATA", &temp)
            .env("LOCALAPPDATA", &temp)
            .env("XDG_CONFIG_HOME", &temp)
            .env("RUST_LOG", "warn")
            .env("HTTPS_PROXY", "http://127.0.0.1:9")
            .env("https_proxy", "http://127.0.0.1:9")
            .env("NO_PROXY", "127.0.0.1,localhost")
            .output()
            .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        if key == "PROJ-FAIL" {
            assert!(!output.status.success());
            assert!(stderr.contains("Failed to update issue"));
        } else {
            assert!(output.status.success(), "{stderr}");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&stdout).unwrap()["key"],
                "PROJ-1"
            );
        }
        let warning = "warning: A link was flattened because Jira wiki cannot represent it.";
        assert_eq!(stderr.matches(warning).count(), usize::from(version == "2"));
        assert!(!stdout.contains(warning));
        assert!(!stderr.contains("Flattening an unrepresentable link"));
    }

    server.join().unwrap();
    std::fs::remove_file(description).unwrap();
    std::fs::remove_dir(temp).unwrap();
}
