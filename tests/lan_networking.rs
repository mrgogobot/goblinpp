use goblinpp::{
    evaluator::Value,
    networking::{self, Network},
    parser,
};
use std::{fs, sync::mpsc, thread};
fn v(s: &str) -> Value {
    Value::Text(s.into())
}
fn text(v: Value) -> String {
    if let Value::Text(s) = v {
        s
    } else {
        panic!("expected text")
    }
}
#[test]
fn pair_has_only_role_keys_private_permissions_and_no_retained_ca_key() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("pair");
    networking::pair_credentials(&root, "192.168.1.42").unwrap();
    for role in ["server", "client"] {
        let mut names = fs::read_dir(root.join(role))
            .unwrap()
            .map(|p| p.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        names.sort();
        let mut expected = vec![
            "CA_SHA256.txt".to_owned(),
            "SERVER_IP.txt".to_owned(),
            "ca.pem".to_owned(),
            format!("{role}.pem"),
            format!("{role}-key.pem"),
        ];
        expected.sort();
        assert_eq!(names, expected);
        assert_eq!(
            fs::read_to_string(root.join(role).join("SERVER_IP.txt")).unwrap(),
            "192.168.1.42"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(root.join(role)).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(root.join(role).join(format!("{role}-key.pem")))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
    assert_eq!(
        fs::read(root.join("client/ca.pem")).unwrap(),
        fs::read(root.join("server/ca.pem")).unwrap()
    );
    assert_ne!(
        fs::read(root.join("client/client-key.pem")).unwrap(),
        fs::read(root.join("server/server-key.pem")).unwrap()
    );
    assert!(networking::pair_credentials(&root, "192.168.1.42").is_err());
    for (i, ip) in [
        "0.0.0.0",
        "224.0.0.1",
        "255.255.255.255",
        "8.8.8.8",
        "192.168.1.1:7443",
        "localhost",
        "::1",
    ]
    .iter()
    .enumerate()
    {
        let out = d.path().join(format!("bad{i}"));
        assert!(networking::pair_credentials(&out, ip).is_err());
        assert!(!out.exists());
    }
}
#[test]
fn dedicated_pair_connects_by_ip_but_wrong_ip_identity_fails() {
    for wrong_name in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let pair = d.path().join("pair");
        networking::pair_credentials(&pair, "127.0.0.1").unwrap();
        let server_base = pair.join("server");
        let client_base = pair.join("client");
        let mut server = Network::default();
        let listener = text(
            server
                .call(
                    &server_base,
                    "net_listen",
                    &[
                        v("127.0.0.1:0"),
                        v("server.pem"),
                        v("server-key.pem"),
                        v("ca.pem"),
                    ],
                )
                .unwrap(),
        );
        let addr = text(
            server
                .call(&server_base, "net_address", &[v(&listener)])
                .unwrap(),
        );
        let (tx, rx) = mpsc::channel();
        let t = thread::spawn(move || {
            let result = server.call(&server_base, "net_accept", &[v(&listener)]);
            if let Ok(channel) = &result {
                server
                    .call(&server_base, "net_send", &[channel.clone(), v("verified")])
                    .unwrap();
            }
            tx.send(result.is_ok()).unwrap();
        });
        let mut client = Network::default();
        let result = client.call(
            &client_base,
            "net_connect",
            &[
                v(&addr),
                v(if wrong_name { "127.0.0.2" } else { "127.0.0.1" }),
                v("ca.pem"),
                v("client.pem"),
                v("client-key.pem"),
            ],
        );
        if wrong_name {
            assert!(result.is_err());
            assert!(!rx.recv().unwrap());
        } else {
            let channel = result.unwrap();
            assert_eq!(
                text(
                    client
                        .call(&client_base, "net_receive", &[channel])
                        .unwrap()
                ),
                "verified"
            );
            assert!(rx.recv().unwrap());
        }
        t.join().unwrap();
    }
}
#[test]
fn old_alpha44_user_function_still_canonicalizes_but_is_now_reserved() {
    let mode=parser::evidence_mode(&serde_json::json!({"goblin_version":"0.1.0-alpha.44","parser_policy":parser::PARSER_POLICY})).unwrap();
    assert_eq!(mode, parser::SyntaxMode::PreLan);
    let source = "g_func net_pair_credentials(x) { return x }\na=net_pair_credentials(2)\nseal a\n";
    assert!(parser::parse_source_with_mode(source, mode).is_ok());
    assert!(
        parser::parse_source(source)
            .and_then(|p| parser::validate_execution(&p.program))
            .is_err()
    );
}

#[test]
fn client_certificate_cannot_impersonate_server_even_under_the_same_ca() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("pair");
    networking::pair_credentials(&root, "127.0.0.1").unwrap();
    let base = root.join("client");
    let mut fake_server = Network::default();
    let listener = text(
        fake_server
            .call(
                &base,
                "net_listen",
                &[
                    v("127.0.0.1:0"),
                    v("client.pem"),
                    v("client-key.pem"),
                    v("ca.pem"),
                ],
            )
            .unwrap(),
    );
    let address = text(
        fake_server
            .call(&base, "net_address", &[v(&listener)])
            .unwrap(),
    );
    let sb = base.clone();
    let t = thread::spawn(move || {
        fake_server
            .call(&sb, "net_accept", &[v(&listener)])
            .is_err()
    });
    let mut client = Network::default();
    assert!(
        client
            .call(
                &base,
                "net_connect",
                &[
                    v(&address),
                    v("127.0.0.1"),
                    v("ca.pem"),
                    v("client.pem"),
                    v("client-key.pem")
                ]
            )
            .is_err()
    );
    assert!(t.join().unwrap());
}
