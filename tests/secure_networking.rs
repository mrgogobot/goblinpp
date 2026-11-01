use goblinpp::{
    evaluator::Value,
    networking::{self, Network},
    parser, recipient_crypto as e2e,
};
use std::{fs, path::Path, sync::mpsc, thread};
use tempfile::tempdir;

fn texts(xs: &[&str]) -> Vec<Value> {
    xs.iter().map(|x| Value::Text((*x).into())).collect()
}
fn call(n: &mut Network, base: &Path, name: &str, xs: &[&str]) -> Value {
    n.call(base, name, &texts(xs)).unwrap()
}
fn string(v: Value) -> String {
    match v {
        Value::Text(s) => s,
        _ => panic!("expected text"),
    }
}
fn listener(n: &mut Network, dir: &Path) -> (String, String) {
    let handle = string(call(
        n,
        dir,
        "net_listen",
        &["127.0.0.1:0", "server.pem", "server-key.pem", "ca.pem"],
    ));
    let address = string(call(n, dir, "net_address", &[&handle]));
    (handle, address)
}
fn connect(
    n: &mut Network,
    dir: &Path,
    address: &str,
    server_name: &str,
) -> goblinpp::error::Result<Value> {
    n.call(
        dir,
        "net_connect",
        &texts(&[
            address,
            server_name,
            "ca.pem",
            "client.pem",
            "client-key.pem",
        ]),
    )
}

#[test]
fn recipient_keys_roundtrip_randomized_and_standard_age_interoperability() {
    use age::secrecy::ExposeSecret;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::Read;
    let d = tempdir().unwrap();
    let path = d.path().join("key");
    let recipient = e2e::keygen(&path).unwrap();
    assert_eq!(e2e::public_key(&path).unwrap(), recipient);
    let a = e2e::encrypt("secret π {{x}}", &recipient).unwrap();
    let b = e2e::encrypt("secret π {{x}}", &recipient).unwrap();
    assert_ne!(a, b);
    assert_eq!(e2e::decrypt(&a, &path).unwrap(), "secret π {{x}}");
    let key: age::x25519::Identity = fs::read_to_string(&path).unwrap().parse().unwrap();
    assert!(
        key.to_string()
            .expose_secret()
            .starts_with("AGE-SECRET-KEY-")
    );
    let bytes = STANDARD.decode(a).unwrap();
    assert!(bytes.starts_with(b"age-encryption.org/v1\n"));
    let mut reader = age::Decryptor::new(bytes.as_slice())
        .unwrap()
        .decrypt(std::iter::once(&key as &dyn age::Identity))
        .unwrap();
    let mut text = String::new();
    reader.read_to_string(&mut text).unwrap();
    assert_eq!(text, "secret π {{x}}");
    for text in ["".to_owned(), "x".repeat(e2e::MAX_TEXT)] {
        assert_eq!(
            e2e::decrypt(&e2e::encrypt(&text, &recipient).unwrap(), &path).unwrap(),
            text
        );
    }
    assert!(e2e::encrypt(&"x".repeat(e2e::MAX_TEXT + 1), &recipient).is_err());
}
#[test]
fn recipient_wrong_key_tampering_truncation_no_clobber_and_limits() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let d = tempdir().unwrap();
    let key = d.path().join("key");
    let other = d.path().join("other");
    let recipient = e2e::keygen(&key).unwrap();
    e2e::keygen(&other).unwrap();
    let ciphertext = e2e::encrypt("secret", &recipient).unwrap();
    assert!(e2e::decrypt(&ciphertext, &other).is_err());
    let bytes = STANDARD.decode(ciphertext).unwrap();
    let mut trailing = bytes.clone();
    trailing.extend_from_slice(b"extra bytes");
    assert!(e2e::decrypt(&STANDARD.encode(trailing), &key).is_err());
    for end in [0, 1, 50, bytes.len() - 1] {
        assert!(e2e::decrypt(&STANDARD.encode(&bytes[..end]), &key).is_err());
    }
    let mut bytes = bytes;
    *bytes.last_mut().unwrap() ^= 1;
    assert!(e2e::decrypt(&STANDARD.encode(bytes), &key).is_err());
    let original = fs::read(&key).unwrap();
    assert!(e2e::keygen(&key).is_err());
    assert_eq!(fs::read(&key).unwrap(), original);
    assert!(e2e::encrypt("x", "not-a-key").is_err());
    assert!(e2e::decrypt(&"x".repeat(e2e::MAX_ENVELOPE + 1), &key).is_err());
}

#[test]
fn password_envelopes_and_excessive_headers_never_invoke_remote_kdf() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::Write;
    let d = tempdir().unwrap();
    let key = d.path().join("key");
    e2e::keygen(&key).unwrap();
    let mut recipient = age::scrypt::Recipient::new(age::secrecy::SecretString::from(
        "public fixture".to_string(),
    ));
    recipient.set_work_factor(1);
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .unwrap();
    let mut bytes = Vec::new();
    let mut w = encryptor.wrap_output(&mut bytes).unwrap();
    w.write_all(b"public").unwrap();
    w.finish().unwrap();
    assert!(e2e::decrypt(&STANDARD.encode(bytes), &key).is_err());
    let oversized_header = format!(
        "age-encryption.org/v1\n{}\n--- {}\n",
        "x".repeat(65536),
        "A".repeat(43)
    );
    assert!(e2e::decrypt(&STANDARD.encode(oversized_header), &key).is_err());
}
#[cfg(unix)]
#[test]
fn private_keys_require_owner_permissions_and_refuse_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let d = tempdir().unwrap();
    let key = d.path().join("key");
    e2e::keygen(&key).unwrap();
    assert_eq!(
        fs::metadata(&key).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(e2e::public_key(&key).is_err());
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let link = d.path().join("link");
    symlink(&key, &link).unwrap();
    assert!(e2e::public_key(&link).is_err());
    assert!(e2e::keygen(&link).is_err());
}

#[test]
fn mtls_roundtrip_utf8_empty_maximum_and_connections_close() {
    let d = tempdir().unwrap();
    networking::demo_credentials(&d.path().join("creds")).unwrap();
    let dir = d.path().join("creds");
    let mut server = Network::default();
    let (listen, addr) = listener(&mut server, &dir);
    let sd = dir.clone();
    let t = thread::spawn(move || {
        let c = string(call(&mut server, &sd, "net_accept", &[&listen]));
        for _ in 0..3 {
            let message = string(call(&mut server, &sd, "net_receive", &[&c]));
            call(&mut server, &sd, "net_send", &[&c, &message]);
        }
        call(&mut server, &sd, "net_close", &[&c]);
        assert!(server.call(&sd, "net_receive", &texts(&[&c])).is_err());
    });
    let mut client = Network::default();
    let c = string(connect(&mut client, &dir, &addr, "localhost").unwrap());
    let fingerprint = string(call(&mut client, &dir, "net_peer_sha256", &[&c]));
    assert_eq!(fingerprint.len(), 64);
    assert!(fingerprint.bytes().all(|b| b.is_ascii_hexdigit()));
    for message in [
        "".into(),
        "hello π\nworld".into(),
        "x".repeat(networking::MAX_MESSAGE),
    ] {
        call(&mut client, &dir, "net_send", &[&c, &message]);
        assert_eq!(
            string(call(&mut client, &dir, "net_receive", &[&c])),
            message
        );
    }
    assert!(
        client
            .call(
                &dir,
                "net_send",
                &texts(&[&c, &"x".repeat(networking::MAX_MESSAGE + 1)])
            )
            .is_err()
    );
    call(&mut client, &dir, "net_close", &[&c]);
    t.join().unwrap();
}

#[test]
fn tls_wrong_name_wrong_authority_and_wrong_client_refuse() {
    let d = tempdir().unwrap();
    let a = d.path().join("a");
    let b = d.path().join("b");
    networking::demo_credentials(&a).unwrap();
    networking::demo_credentials(&b).unwrap();
    for case in 0..3 {
        let mut server = Network::default();
        server
            .call(
                &a,
                "net_timeout",
                &[Value::Quantity(
                    goblinpp::quantity::Quantity::scalar(1000.).unwrap(),
                )],
            )
            .unwrap();
        let (listen, addr) = listener(&mut server, &a);
        let sd = a.clone();
        let t = thread::spawn(move || {
            server
                .call(&sd, "net_accept", &texts(&[&listen]))
                .and_then(|v| server.call(&sd, "net_receive", &texts(&[&string(v)])))
        });
        let mut client = Network::default();
        let base = if case == 1 { &b } else { &a };
        let result = if case == 2 {
            client.call(
                base,
                "net_connect",
                &texts(&[
                    &addr,
                    "localhost",
                    "ca.pem",
                    b.join("client.pem").to_str().unwrap(),
                    b.join("client-key.pem").to_str().unwrap(),
                ]),
            )
        } else {
            connect(
                &mut client,
                base,
                &addr,
                if case == 0 {
                    "wrong.example"
                } else {
                    "localhost"
                },
            )
        };
        if let Ok(v) = result {
            let c = string(v);
            let _ = client.call(base, "net_send", &texts(&[&c, "must not arrive"]));
            assert!(client.call(base, "net_receive", &texts(&[&c])).is_err());
        }
        assert!(t.join().unwrap().is_err());
    }
}
#[test]
fn address_policy_timeouts_resource_limits_and_demo_no_clobber() {
    let d = tempdir().unwrap();
    let mut network = Network::default();
    for addr in [
        "0.0.0.0:7443",
        "[::]:7443",
        "224.0.0.1:7443",
        "255.255.255.255:7443",
        "localhost:7443",
    ] {
        assert!(
            network
                .call(d.path(), "net_allow", &texts(&[addr]))
                .is_err()
        );
    }
    let args = texts(&["192.0.2.1:7443", "localhost", "a", "b", "c"]);
    assert!(
        network
            .call(d.path(), "net_connect", &args)
            .unwrap_err()
            .to_string()
            .contains("net_allow")
    );
    call(&mut network, d.path(), "net_allow", &["192.0.2.1:7443"]);
    assert!(
        !network
            .call(d.path(), "net_connect", &args)
            .unwrap_err()
            .to_string()
            .contains("net_allow")
    );
    for ms in [0., -1., 1.5, 60_001.] {
        assert!(
            network
                .call(
                    d.path(),
                    "net_timeout",
                    &[Value::Quantity(
                        goblinpp::quantity::Quantity::scalar(ms).unwrap()
                    )]
                )
                .is_err()
        );
    }
    let dir = d.path().join("credentials");
    networking::demo_credentials(&dir).unwrap();
    assert!(networking::demo_credentials(&dir).is_err());
    network
        .call(
            &dir,
            "net_timeout",
            &[Value::Quantity(
                goblinpp::quantity::Quantity::scalar(20.).unwrap(),
            )],
        )
        .unwrap();
    let (listen, _) = listener(&mut network, &dir);
    let start = std::time::Instant::now();
    assert!(
        network
            .call(&dir, "net_accept", &texts(&[&listen]))
            .is_err()
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    for _ in 1..32 {
        listener(&mut network, &dir);
    }
    assert!(
        network
            .call(
                &dir,
                "net_listen",
                &texts(&["127.0.0.1:0", "server.pem", "server-key.pem", "ca.pem"])
            )
            .is_err()
    );
}

#[test]
fn static_privacy_arity_discard_and_history() {
    for source in [
        "x = net_receive(\"x\")",
        "GO_PARANOID\nx = e2e_encrypt(\"secret\", \"key\")",
        "GO_PRIVATE\nnet_connect(\"x\",\"x\",\"x\",\"x\",\"x\")",
        "GO_PRIVATE\ne2e_keygen(\"key\")",
        "GO_PRIVATE\ng_func net_send(x) { return x }",
    ] {
        assert!(
            parser::parse_source(source)
                .and_then(|p| parser::validate_execution(&p.program))
                .is_err(),
            "{source}"
        );
    }
    let mode = parser::evidence_mode(&serde_json::json!({"goblin_version":"0.1.0-alpha.43", "parser_policy":parser::PARSER_POLICY})).unwrap();
    assert_eq!(mode, parser::SyntaxMode::PreNetwork);
    // Genuine historical user functions must still canonicalize, but not execute as new builtins.
    assert!(
        parser::parse_source_with_mode("g_func net_send(x) { return x }\na = net_send(1)\n", mode)
            .is_ok()
    );
}

#[test]
fn encrypted_relay_carries_only_ciphertext() {
    let d = tempdir().unwrap();
    let recipient_key = d.path().join("recipient");
    let recipient = e2e::keygen(&recipient_key).unwrap();
    let cipher = e2e::encrypt("private science discussion", &recipient).unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        assert!(!cipher.contains("private science discussion"));
        tx.send(cipher).unwrap();
    })
    .join()
    .unwrap();
    assert_eq!(
        e2e::decrypt(&rx.recv().unwrap(), &recipient_key).unwrap(),
        "private science discussion"
    );
}

#[test]
fn cli_capabilities_exposes_private_network_and_recipient_contracts() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_goblinpp"))
        .args(["capabilities", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let caps: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    for name in [
        "net_demo_credentials",
        "net_pair_credentials",
        "net_allow",
        "net_timeout",
        "net_listen",
        "net_connect",
        "net_accept",
        "net_send",
        "net_receive",
        "net_close",
        "net_address",
        "net_peer_sha256",
        "e2e_keygen",
        "e2e_public_key",
        "e2e_encrypt",
        "e2e_decrypt",
    ] {
        assert!(
            caps["confidential_functions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == name)
        );
    }
    assert_eq!(
        caps["secure_network"]["tls_versions"],
        serde_json::json!(["TLS1.3"])
    );
    assert_eq!(caps["secure_network"]["client_authentication"], "REQUIRED");
    assert_eq!(caps["recipient_encryption"]["sender_signatures"], false);
}
