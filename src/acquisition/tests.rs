use super::*;
use std::fs;
use std::net::TcpListener;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::thread::JoinHandle;

const BODY: &[u8] =
    b"SIMPLE  = T\0\xff\x80\nThis is a binary transport fixture, not a valid FITS cube.\n";

fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn request() -> FetchRequest {
    FetchRequest {
        url: "https://download.example.org/data?token=never-log-me".into(),
        allow_host: "download.example.org".into(),
        expected_sha256: Some(sha256_bytes(BODY)),
        record_only: false,
        filename: "cube.fits".into(),
        max_bytes: 1024,
        timeout_seconds: 3,
    }
}

fn response(body: &[u8]) -> Vec<u8> {
    let mut result = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Disposition: attachment; filename=untrusted.exe\r\nETag: private-response-tag\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
    result.extend_from_slice(body);
    result
}

struct Fixture {
    address: SocketAddr,
    certificate: Vec<u8>,
    thread: JoinHandle<Vec<u8>>,
}

fn server(wire: Vec<u8>, delay: Duration, name: &str, expired: bool) -> Fixture {
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(vec![name.into()]).unwrap();
    if expired {
        params.not_before = rcgen::date_time_ymd(2000, 1, 1);
        params.not_after = rcgen::date_time_ymd(2001, 1, 1);
    }
    let cert = params.self_signed(&key).unwrap();
    let certificate = cert.der().to_vec();
    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.der().clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let thread = std::thread::spawn(move || {
        let start = Instant::now();
        let stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        && start.elapsed() < Duration::from_secs(5) =>
                {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(_) => return vec![],
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
        let mut stream = rustls::StreamOwned::new(connection, stream);
        let mut input = Vec::new();
        let mut b = [0u8; 1];
        while input.len() < HEADER_BYTES && !input.ends_with(b"\r\n\r\n") {
            match stream.read(&mut b) {
                Ok(1) => input.push(b[0]),
                _ => return input,
            }
        }
        let split = wire
            .windows(4)
            .position(|p| p == b"\r\n\r\n")
            .map(|p| p + 4)
            .unwrap_or(wire.len());
        if stream.write_all(&wire[..split]).is_ok() {
            let _ = stream.flush();
            std::thread::sleep(delay);
            let _ = stream.write_all(&wire[split..]);
            let _ = stream.flush();
            stream.conn.send_close_notify();
            let _ = stream.flush();
        }
        input
    });
    Fixture {
        address,
        certificate,
        thread,
    }
}

fn acquire_fixture(
    r: &FetchRequest,
    out: &Path,
    fixture: Fixture,
    trust: bool,
) -> (Result<AcquisitionReceipt>, Vec<u8>) {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let resolver = PublicResolver {
        trace: trace.clone(),
        test_address: Some(fixture.address),
    };
    let roots = if trust {
        RootCerts::new_with_certs(&[
            ureq::tls::Certificate::from_der(&fixture.certificate).to_owned()
        ])
    } else {
        RootCerts::PlatformVerifier
    };
    let client = agent(r, resolver, roots);
    let result = acquire(r, &r.validate().unwrap(), out, &client, &trace);
    (result, fixture.thread.join().unwrap())
}

fn good(out: &Path) -> AcquisitionReceipt {
    let f = server(
        response(BODY),
        Duration::ZERO,
        "download.example.org",
        false,
    );
    acquire_fixture(&request(), out, f, true).0.unwrap()
}

fn rewrite_receipt(out: &Path, change: impl FnOnce(&mut serde_json::Value)) {
    let path = out.join(RECEIPT);
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    change(&mut v);
    v.as_object_mut().unwrap().remove("receipt_core_sha256");
    v["receipt_core_sha256"] = hash_canonical_json(&v).unwrap().into();
    fs::write(path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}

#[test]
fn insecure_urls_credentials_ip_literals_and_implicit_hosts_refuse_before_output() {
    let dir = tempdir();
    for url in [
        "http://download.example.org/file",
        "file:///etc/passwd",
        "ftp://download.example.org/file",
        "https://user:secret@download.example.org/file",
        "https://127.0.0.1/",
        "https://[::1]/",
        "https://2130706433/",
        "https://localhost/",
        "https://host.local/",
        "https://host.internal/",
        "https://download.example.org:8443/",
        "https://download.example.org/#secret",
        "https://download.example.org/a\r\nX-Evil: yes",
        "https://downlоad.example.org/",
        "https://download.example.org\\@localhost/",
    ] {
        let mut r = request();
        r.url = url.into();
        let out = dir.path().join("refused");
        assert!(fetch(&r, &out).is_err(), "URL should be refused");
        assert!(!out.exists());
    }
    for host in [
        "other.example.org",
        "*.example.org",
        "DOWNLOAD.example.org",
        "download.example.org.",
    ] {
        let mut r = request();
        r.allow_host = host.into();
        assert!(r.validate().is_err());
    }
}

#[test]
fn nonpublic_and_transition_ip_ranges_are_refused() {
    for address in [
        "0.0.0.0",
        "10.1.2.3",
        "100.64.1.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.31.1.1",
        "192.0.0.1",
        "192.0.2.1",
        "192.88.99.1",
        "192.168.1.1",
        "198.18.1.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "::ffff:8.8.8.8",
        "fc00::1",
        "fe80::1",
        "ff02::1",
        "64:ff9b::1",
        "2001:db8::1",
        "2001::1",
        "2001:20::1",
        "2002:0808:0808::1",
        "3fff::1",
    ] {
        assert!(!public_ip(address.parse().unwrap()), "{address}");
    }
    for address in [
        "8.8.8.8",
        "1.1.1.1",
        "93.184.216.34",
        "2606:4700::1111",
        "2001:4860:4860::8888",
    ] {
        assert!(public_ip(address.parse().unwrap()));
    }
}

#[test]
fn explicit_integrity_limits_and_safe_names_are_required() {
    let mut r = request();
    r.expected_sha256 = None;
    assert!(r.validate().is_err());
    r.record_only = true;
    assert!(r.validate().is_ok());
    r.expected_sha256 = Some(sha256_bytes(BODY));
    assert!(r.validate().is_err());
    r.record_only = false;
    r.expected_sha256 = Some("A".repeat(64));
    assert!(r.validate().is_err());
    for name in [
        "",
        "..",
        "../outside",
        "a/b",
        "a\\b",
        ".hidden",
        "a.",
        RECEIPT,
        PARTIAL,
        "CON",
        "nul.txt",
        "x:stream",
    ] {
        let mut r = request();
        r.filename = name.into();
        assert!(r.validate().is_err(), "{name}");
    }
    for limit in [0, MAX_BYTES + 1, u64::MAX] {
        let mut r = request();
        r.max_bytes = limit;
        assert!(r.validate().is_err());
    }
    for limit in [0, MAX_TIMEOUT_SECONDS + 1] {
        let mut r = request();
        r.timeout_seconds = limit;
        assert!(r.validate().is_err());
    }
}

#[test]
fn secure_configuration_disables_proxies_redirects_and_implicit_decoding() {
    let r = request();
    let resolver = PublicResolver {
        trace: Arc::new(Mutex::new(Trace::default())),
        test_address: None,
    };
    let a = agent(&r, resolver, RootCerts::PlatformVerifier);
    assert!(a.config().https_only());
    assert!(a.config().proxy().is_none());
    assert_eq!(a.config().max_redirects(), 0);
    assert_eq!(a.config().max_response_header_size(), HEADER_BYTES);
}

#[test]
fn binary_payload_success_is_exact_private_and_offline_verifiable() {
    let dir = tempdir();
    let out = dir.path().join("acquired");
    let f = server(
        response(BODY),
        Duration::ZERO,
        "download.example.org",
        false,
    );
    let (receipt, request_bytes) = acquire_fixture(&request(), &out, f, true);
    let receipt = receipt.unwrap();
    assert_eq!(receipt.status, AcquisitionStatus::CompleteHashVerified);
    assert_eq!(fs::read(out.join("cube.fits")).unwrap(), BODY);
    assert!(!out.join(PARTIAL).exists());
    assert!(!out.join("untrusted.exe").exists());
    assert_eq!(
        fs::metadata(&out).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(out.join("cube.fits"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o400
    );
    assert_eq!(
        fs::metadata(out.join(RECEIPT))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(
        String::from_utf8_lossy(&request_bytes).contains("GET /data?token=never-log-me HTTP/1.1")
    );
    let request_bytes = String::from_utf8_lossy(&request_bytes).to_lowercase();
    assert!(request_bytes.contains("accept-encoding: identity"));
    assert!(!request_bytes.contains("authorization:"));
    assert!(!request_bytes.contains("cookie:"));
    let record = fs::read_to_string(out.join(RECEIPT)).unwrap();
    assert!(!record.contains("never-log-me"));
    assert!(!record.contains("private-response-tag"));
    assert_eq!(
        verify(&out, Some(&receipt.receipt_core_sha256))
            .unwrap()
            .payload_sha256,
        sha256_bytes(BODY)
    );
    assert!(verify(&out, Some(&"0".repeat(64))).is_err());
}

#[test]
fn observed_only_is_not_promoted_to_hash_verified() {
    let d = tempdir();
    let out = d.path().join("observed");
    let mut r = request();
    r.record_only = true;
    r.expected_sha256 = None;
    let f = server(
        response(BODY),
        Duration::ZERO,
        "download.example.org",
        false,
    );
    assert_eq!(
        acquire_fixture(&r, &out, f, true).0.unwrap().status,
        AcquisitionStatus::CompleteObservedOnly
    );
    rewrite_receipt(&out, |v| v["status"] = "COMPLETE_HASH_VERIFIED".into());
    assert!(verify(&out, None).is_err());
}

#[test]
fn tls_unknown_root_wrong_hostname_and_expired_certificates_refuse() {
    for (name, expired, trust) in [
        ("download.example.org", false, false),
        ("wrong.example.org", false, true),
        ("download.example.org", true, true),
    ] {
        let d = tempdir();
        let out = d.path().join("refused");
        let f = server(response(BODY), Duration::ZERO, name, expired);
        let error = acquire_fixture(&request(), &out, f, trust).0.unwrap_err();
        assert!(!error.pretty().contains("never-log-me"));
        let r = verify(&out, None).unwrap();
        assert_eq!(r.status, AcquisitionStatus::Failed);
        assert_eq!(r.failure.as_deref(), Some("HTTPS_REQUEST_FAILED"));
        assert!(!out.join("cube.fits").exists());
    }
}

#[test]
fn redirects_status_errors_and_partial_responses_never_succeed() {
    for wire in [b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/secret?token=never-log-me\r\nContent-Length: 0\r\n\r\n".to_vec(),
        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_vec(),
        b"HTTP/1.1 206 Partial Content\r\nContent-Length: 0\r\n\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Range: bytes 0-1/10\r\nContent-Length: 0\r\n\r\n".to_vec()] {
        let d = tempdir(); let out = d.path().join("refused");
        let f = server(wire, Duration::ZERO, "download.example.org", false);
        assert!(acquire_fixture(&request(), &out, f, true).0.is_err());
        assert_eq!(verify(&out, None).unwrap().status, AcquisitionStatus::Failed);
        assert!(!fs::read_to_string(out.join(RECEIPT)).unwrap().contains("never-log-me"));
    }
}

#[test]
fn content_coding_ambiguous_framing_and_truncation_refuse() {
    for wire in [b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 0\r\n\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nab".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n0\r\n\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nshort".to_vec()] {
        let d = tempdir(); let out = d.path().join("refused");
        let f = server(wire, Duration::ZERO, "download.example.org", false);
        assert!(acquire_fixture(&request(), &out, f, true).0.is_err());
        assert_eq!(verify(&out, None).unwrap().status, AcquisitionStatus::Failed);
    }
}

#[test]
fn expected_hash_mismatch_keeps_only_labelled_partial_evidence() {
    let d = tempdir();
    let out = d.path().join("refused");
    let mut r = request();
    r.expected_sha256 = Some("0".repeat(64));
    let f = server(
        response(BODY),
        Duration::ZERO,
        "download.example.org",
        false,
    );
    assert!(acquire_fixture(&r, &out, f, true).0.is_err());
    let receipt = verify(&out, None).unwrap();
    assert_eq!(receipt.failure.as_deref(), Some("EXPECTED_SHA256_MISMATCH"));
    assert_eq!(fs::read(out.join(PARTIAL)).unwrap(), BODY);
    assert!(!out.join("cube.fits").exists());
}

#[test]
fn size_limits_count_unknown_length_bodies_and_accept_exact_boundary() {
    for (headers, body, limit, success) in [
        ("Content-Length: 4\r\n", b"1234".as_slice(), 4, true),
        ("Content-Length: 5\r\n", b"12345".as_slice(), 4, false),
        ("", b"12345".as_slice(), 4, false),
        (
            "Transfer-Encoding: chunked\r\n",
            b"5\r\n12345\r\n0\r\n\r\n".as_slice(),
            4,
            false,
        ),
    ] {
        let d = tempdir();
        let out = d.path().join("limited");
        let mut r = request();
        r.record_only = true;
        r.expected_sha256 = None;
        r.max_bytes = limit;
        let mut wire =
            format!("HTTP/1.1 200 OK\r\n{headers}Connection: close\r\n\r\n").into_bytes();
        wire.extend_from_slice(body);
        let f = server(wire, Duration::ZERO, "download.example.org", false);
        assert_eq!(acquire_fixture(&r, &out, f, true).0.is_ok(), success);
        let receipt = verify(&out, None).unwrap();
        assert!(receipt.payload_bytes <= limit);
    }
}

#[test]
fn transfer_deadline_preserves_failure_instead_of_hanging() {
    let d = tempdir();
    let out = d.path().join("deadline");
    let mut r = request();
    r.timeout_seconds = 1;
    let f = server(
        response(BODY),
        Duration::from_millis(1400),
        "download.example.org",
        false,
    );
    assert!(acquire_fixture(&r, &out, f, true).0.is_err());
    assert_eq!(
        verify(&out, None).unwrap().status,
        AcquisitionStatus::Failed
    );
}

#[test]
fn payload_receipt_policy_and_extra_file_tampering_refuse() {
    let d = tempdir();
    let out = d.path().join("original");
    good(&out);
    let original = fs::read(out.join(RECEIPT)).unwrap();
    for field in [
        "policy",
        "client",
        "source_url_sha256",
        "payload_sha256",
        "allowed_host",
        "status",
    ] {
        rewrite_receipt(&out, |v| v[field] = "tampered".into());
        assert!(verify(&out, None).is_err());
        fs::write(out.join(RECEIPT), &original).unwrap();
    }
    rewrite_receipt(&out, |v| {
        v["security"]["public_addresses_only"] = false.into()
    });
    assert!(verify(&out, None).is_err());
    fs::write(out.join(RECEIPT), &original).unwrap();
    rewrite_receipt(&out, |v| {
        v["validated_dns_addresses"] = serde_json::json!(["127.0.0.1:443"])
    });
    assert!(verify(&out, None).is_err());
    fs::write(out.join(RECEIPT), &original).unwrap();
    rewrite_receipt(&out, |v| {
        v["validated_dns_addresses"] = serde_json::json!([])
    });
    assert!(verify(&out, None).is_err());
    fs::write(out.join(RECEIPT), &original).unwrap();
    rewrite_receipt(&out, |v| v["extra"] = true.into());
    assert!(verify(&out, None).is_err());
    fs::write(out.join(RECEIPT), &original).unwrap();
    fs::write(out.join("extra"), b"not inventoried").unwrap();
    assert!(verify(&out, None).is_err());
    fs::remove_file(out.join("extra")).unwrap();
    fs::set_permissions(out.join("cube.fits"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(out.join("cube.fits"), b"changed").unwrap();
    assert!(verify(&out, None).is_err());
}

#[test]
fn existing_paths_symlinks_hardlinks_and_special_files_are_not_followed() {
    let d = tempdir();
    let existing = d.path().join("existing");
    fs::create_dir(&existing).unwrap();
    assert!(Bundle::create(&existing).is_err());
    symlink(&existing, d.path().join("link")).unwrap();
    assert!(Bundle::create(&d.path().join("link/new")).is_err());
    assert!(Bundle::open(&d.path().join("link")).is_err());
    assert!(Bundle::create(&d.path().join("../outside")).is_err());
    let bundle = Bundle::create(&d.path().join("files")).unwrap();
    bundle.create_file("one").unwrap();
    assert!(bundle.create_file("one").is_err());
    fs::hard_link(d.path().join("files/one"), d.path().join("files/two")).unwrap();
    assert!(bundle.read_file("one").is_err());
    symlink(d.path().join("files/one"), d.path().join("files/alias")).unwrap();
    assert!(bundle.read_file("alias").is_err());
    let fifo = std::ffi::CString::new(d.path().join("files/fifo").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert!(bundle.read_file("fifo").is_err());
}

#[test]
fn changed_output_directory_and_publication_collision_refuse() {
    let d = tempdir();
    let out = d.path().join("bundle");
    let b = Bundle::create(&out).unwrap();
    b.create_file(PARTIAL).unwrap();
    b.create_file("cube.fits").unwrap();
    assert!(b.publish(PARTIAL, "cube.fits").is_err());
    fs::rename(&out, d.path().join("moved")).unwrap();
    fs::create_dir(&out).unwrap();
    assert!(b.finish().is_err());
    assert!(b.create_file("new").is_err());
}

#[test]
fn oversized_receipts_and_path_redirection_refuse() {
    let d = tempdir();
    let out = d.path().join("bundle");
    good(&out);
    let original = fs::read(out.join(RECEIPT)).unwrap();
    rewrite_receipt(&out, |v| v["payload_path"] = "../outside".into());
    assert!(verify(&out, None).is_err());
    fs::write(out.join(RECEIPT), &original).unwrap();
    fs::write(out.join(RECEIPT), vec![b' '; RECEIPT_BYTES as usize + 1]).unwrap();
    assert!(verify(&out, None).is_err());
}

#[test]
fn dns_candidates_are_all_checked_and_ports_are_pinned() {
    let public: SocketAddr = "1.1.1.1:443".parse().unwrap();
    assert!(vetted_addresses(&[public]));
    assert!(!vetted_addresses(&[]));
    assert!(!vetted_addresses(&[
        public,
        "127.0.0.1:443".parse().unwrap()
    ]));
    assert!(!vetted_addresses(&["1.1.1.1:80".parse().unwrap()]));
    assert!(!vetted_addresses(&vec![public; 17]));
}

#[test]
fn oversized_headers_fail_before_body_publication() {
    let d = tempdir();
    let out = d.path().join("headers");
    let wire = format!(
        "HTTP/1.1 200 OK\r\nX-Large: {}\r\nContent-Length: 0\r\n\r\n",
        "x".repeat(HEADER_BYTES + 100)
    )
    .into_bytes();
    let f = server(wire, Duration::ZERO, "download.example.org", false);
    assert!(acquire_fixture(&request(), &out, f, true).0.is_err());
    let receipt = verify(&out, None).unwrap();
    assert_eq!(receipt.status, AcquisitionStatus::Failed);
    assert!(!out.join("cube.fits").exists());
}

#[test]
fn empty_and_larger_binary_payloads_round_trip_without_text_decoding() {
    for body in [
        vec![],
        (0..8 * 1024 * 1024)
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>(),
    ] {
        let d = tempdir();
        let out = d.path().join("binary");
        let mut r = request();
        r.expected_sha256 = Some(sha256_bytes(&body));
        r.max_bytes = body.len().max(1) as u64;
        r.timeout_seconds = 30;
        let f = server(
            response(&body),
            Duration::ZERO,
            "download.example.org",
            false,
        );
        let receipt = acquire_fixture(&r, &out, f, true).0.unwrap();
        assert_eq!(receipt.payload_bytes, body.len() as u64);
        assert_eq!(fs::read(out.join("cube.fits")).unwrap(), body);
    }
}

#[test]
fn close_delimited_and_chunked_messages_preserve_payload_not_http_framing() {
    for wire in [
        b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n1234".to_vec(),
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n12\r\n2\r\n34\r\n0\r\n\r\n"
            .to_vec(),
    ] {
        let d = tempdir();
        let out = d.path().join("framing");
        let mut r = request();
        r.expected_sha256 = Some(sha256_bytes(b"1234"));
        r.max_bytes = 4;
        let f = server(wire, Duration::ZERO, "download.example.org", false);
        assert!(acquire_fixture(&r, &out, f, true).0.is_ok());
        assert_eq!(fs::read(out.join("cube.fits")).unwrap(), b"1234");
    }
}
