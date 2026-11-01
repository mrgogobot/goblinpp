//! Real interpreter and native binaries share the same TLS/E2E implementation.
use goblinpp::{
    compiler,
    evaluator::Value,
    networking::{self, Network},
    parser,
};
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
fn values(xs: &[&str]) -> Vec<Value> {
    xs.iter().map(|s| Value::Text((*s).into())).collect()
}
fn text(value: Value) -> String {
    if let Value::Text(v) = value {
        v
    } else {
        panic!("expected text")
    }
}

#[test]
fn interpreted_and_native_clients_and_servers_exchange_recipient_encrypted_messages_without_receipts()
 {
    let directory = tempfile::tempdir().unwrap();
    let dir = directory.path();
    networking::demo_credentials(&dir.join("creds")).unwrap();
    let public = goblinpp::recipient_crypto::keygen(&dir.join("recipient.key")).unwrap();
    fs::write(dir.join("recipient.txt"), public).unwrap();
    let source = r#"GO_PRIVATE
net_timeout(2000)
if argv(1) == "client" {
    recipient = read_text("recipient.txt")
    ciphertext = e2e_encrypt("native and interpreted π", recipient)
    channel = net_connect(argv(2), "localhost", "creds/ca.pem", "creds/client.pem", "creds/client-key.pem")
    net_send(channel, ciphertext)
    answer = net_receive(channel)
    print(answer)
    net_close(channel)
} else {
    listener = net_listen(argv(2), "creds/server.pem", "creds/server-key.pem", "creds/ca.pem")
    channel = net_accept(listener)
    ciphertext = net_receive(channel)
    message = e2e_decrypt(ciphertext, "recipient.key")
    if message == "native and interpreted π" {
        net_send(channel, "verified")
    } else {
        net_send(channel, "mismatch")
    }
    net_close(channel)
    net_close(listener)
}
"#;
    let source_path = dir.join("network.gbl");
    fs::write(&source_path, source).unwrap();
    let executable = dir.join("native");
    compiler::compile(&parser::parse_source(source).unwrap(), &executable, &[]).unwrap();
    for native in [false, true] {
        // Real native or interpreted client talks to a real Rustls Goblin listener.
        let mut server = Network::default();
        let base = dir.join("creds");
        let listener = text(
            server
                .call(
                    &base,
                    "net_listen",
                    &values(&["127.0.0.1:0", "server.pem", "server-key.pem", "ca.pem"]),
                )
                .unwrap(),
        );
        let addr = text(
            server
                .call(&base, "net_address", &values(&[&listener]))
                .unwrap(),
        );
        let key = dir.join("recipient.key");
        let t = thread::spawn(move || {
            let c = text(
                server
                    .call(&base, "net_accept", &values(&[&listener]))
                    .unwrap(),
            );
            let envelope = text(server.call(&base, "net_receive", &values(&[&c])).unwrap());
            assert_eq!(
                goblinpp::recipient_crypto::decrypt(&envelope, &key).unwrap(),
                "native and interpreted π"
            );
            server
                .call(&base, "net_send", &values(&[&c, "verified"]))
                .unwrap();
        });
        let mut command = Command::new(if native {
            &executable
        } else {
            std::path::Path::new(env!("CARGO_BIN_EXE_goblinpp"))
        });
        if !native {
            command.args(["run", "network.gbl", "--"]);
        }
        let out = command
            .args(["client", &addr])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("verified"));
        t.join().unwrap();

        // Real native/interpreted server decrypts using its private recipient key.
        let temporary = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = temporary.local_addr().unwrap().to_string();
        drop(temporary);
        let mut command = Command::new(if native {
            &executable
        } else {
            std::path::Path::new(env!("CARGO_BIN_EXE_goblinpp"))
        });
        if !native {
            command.args(["run", "network.gbl", "--"]);
        }
        let mut child = command
            .args(["server", &addr])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut client = Network::default();
        let base = dir.join("creds");
        let start = Instant::now();
        let c = loop {
            if let Ok(c) = client.call(
                &base,
                "net_connect",
                &values(&[&addr, "localhost", "ca.pem", "client.pem", "client-key.pem"]),
            ) {
                break text(c);
            }
            if start.elapsed() > Duration::from_secs(3) {
                let _ = child.kill();
                panic!("server did not become ready");
            }
            thread::sleep(Duration::from_millis(5));
        };
        let ciphertext = goblinpp::recipient_crypto::encrypt(
            "native and interpreted π",
            fs::read_to_string(dir.join("recipient.txt"))
                .unwrap()
                .as_str(),
        )
        .unwrap();
        client
            .call(&base, "net_send", &values(&[&c, &ciphertext]))
            .unwrap();
        assert_eq!(
            text(client.call(&base, "net_receive", &values(&[&c])).unwrap()),
            "verified"
        );
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    for path in [
        "runs",
        ".goblin",
        "receipt.json",
        "native-data.json",
        "final-state.json",
    ] {
        assert!(!dir.join(path).exists());
    }
}

#[test]
fn check_examples_is_static_and_creates_no_network_credentials() {
    let d = tempfile::tempdir().unwrap();
    for name in [
        "network_setup.gbl",
        "network_client.gbl",
        "network_server.gbl",
        "network_relay.gbl",
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
            .arg("check")
            .arg(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("examples")
                    .join(name),
            )
            .current_dir(d.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 0);
}

#[test]
fn real_relay_runs_without_recipient_private_key() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path();
    let credentials = dir.join("credentials");
    networking::demo_credentials(&credentials).unwrap();
    let recipient = goblinpp::recipient_crypto::keygen(&dir.join("recipient.key")).unwrap();
    let relay_directory = dir.join("relay");
    fs::create_dir(&relay_directory).unwrap();
    fs::create_dir(relay_directory.join("network-demo")).unwrap();
    for name in [
        "ca.pem",
        "server.pem",
        "server-key.pem",
        "client.pem",
        "client-key.pem",
    ] {
        fs::copy(
            credentials.join(name),
            relay_directory.join("network-demo").join(name),
        )
        .unwrap();
    }
    let mut server = Network::default();
    let listener = text(
        server
            .call(
                &credentials,
                "net_listen",
                &values(&["127.0.0.1:0", "server.pem", "server-key.pem", "ca.pem"]),
            )
            .unwrap(),
    );
    let server_address = text(
        server
            .call(&credentials, "net_address", &values(&[&listener]))
            .unwrap(),
    );
    let key = dir.join("recipient.key");
    let sd = credentials.clone();
    let t = thread::spawn(move || {
        let c = text(
            server
                .call(&sd, "net_accept", &values(&[&listener]))
                .unwrap(),
        );
        let envelope = text(server.call(&sd, "net_receive", &values(&[&c])).unwrap());
        assert_eq!(
            goblinpp::recipient_crypto::decrypt(&envelope, &key).unwrap(),
            "relay cannot decrypt me"
        );
        server
            .call(&sd, "net_send", &values(&[&c, "verified"]))
            .unwrap();
    });
    let temporary = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = temporary.local_addr().unwrap().to_string();
    drop(temporary);
    let source = include_str!("../examples/network_relay.gbl")
        .replace("127.0.0.1:7443", &server_address)
        .replace("127.0.0.1:7444", &addr);
    fs::write(relay_directory.join("relay.gbl"), source).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["run", "relay.gbl"])
        .current_dir(&relay_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut client = Network::default();
    let start = Instant::now();
    let c = loop {
        if let Ok(c) = client.call(
            &credentials,
            "net_connect",
            &values(&[&addr, "localhost", "ca.pem", "client.pem", "client-key.pem"]),
        ) {
            break text(c);
        }
        if start.elapsed() > Duration::from_secs(3) {
            let _ = child.kill();
            panic!("relay did not become ready");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let envelope =
        goblinpp::recipient_crypto::encrypt("relay cannot decrypt me", &recipient).unwrap();
    client
        .call(&credentials, "net_send", &values(&[&c, &envelope]))
        .unwrap();
    assert_eq!(
        text(
            client
                .call(&credentials, "net_receive", &values(&[&c]))
                .unwrap()
        ),
        "verified"
    );
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    t.join().unwrap();
    assert!(!relay_directory.join("network-demo/recipient.key").exists());
    assert!(!relay_directory.join("runs").exists());
    assert!(!relay_directory.join(".goblin").exists());
}
