//! Bounded UTF-8 messages over TLS 1.3 with mandatory mutual certificate authentication.
//! No DNS, plaintext fallback, proxy settings, shell execution or certificate bypass.
use crate::{
    error::{GoblinError, Result},
    evaluator::Value,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, pem::PemObject};
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig, ServerConnection};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

pub const MAX_MESSAGE: usize = 1_048_576;
const MAX_HANDLES: usize = 32;
const MAX_CREATED: u64 = 1024;
const MAX_PEM: u64 = 65_536;
const ALPN: &[u8] = b"goblin-message/1";

fn error(message: &str) -> GoblinError {
    GoblinError::new("G903", message)
}
fn io_error<T>(_: T) -> GoblinError {
    error(
        "Secure network operation failed: check certificates, peer identity, address permission, timeout and message limits. Values and paths are omitted; no plaintext fallback is permitted.",
    )
}
pub fn is_function(name: &str) -> bool {
    matches!(
        name,
        "net_demo_credentials"
            | "net_pair_credentials"
            | "net_allow"
            | "net_timeout"
            | "net_listen"
            | "net_connect"
            | "net_accept"
            | "net_send"
            | "net_receive"
            | "net_close"
            | "net_address"
            | "net_peer_sha256"
    )
}
pub fn is_effect(name: &str) -> bool {
    matches!(
        name,
        "net_demo_credentials"
            | "net_pair_credentials"
            | "net_allow"
            | "net_timeout"
            | "net_send"
            | "net_close"
    )
}
pub fn arity(name: &str, count: usize) -> Result<()> {
    let n = match name {
        "net_connect" => 5,
        "net_listen" => 4,
        "net_send" | "net_pair_credentials" => 2,
        _ => 1,
    };
    if is_function(name) && count == n {
        Ok(())
    } else {
        Err(error("Invalid secure-network argument count."))
    }
}
fn address(text: &str) -> Result<SocketAddr> {
    if text.len() > 80 {
        return Err(error("Use a numeric IP:port address."));
    }
    let addr: SocketAddr = text
        .parse()
        .map_err(|_| error("Use a numeric IP:port address; DNS resolution is not implicit."))?;
    if addr.ip().is_unspecified()
        || addr.ip().is_multicast()
        || addr.ip() == std::net::Ipv4Addr::BROADCAST
    {
        return Err(error(
            "Wildcard, broadcast and multicast addresses are refused.",
        ));
    }
    Ok(addr)
}

/// An absolute operation deadline. Partial traffic cannot reset the time allowance.
#[derive(Debug)]
struct DeadlineSocket {
    socket: TcpStream,
    deadline: Instant,
}
impl DeadlineSocket {
    fn reset(&mut self, timeout: Duration) {
        self.deadline = Instant::now() + timeout;
    }
    fn remaining(&self) -> std::io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|v| !v.is_zero())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "Operation deadline reached.")
            })
    }
}
impl Read for DeadlineSocket {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket.set_read_timeout(Some(self.remaining()?))?;
        self.socket.read(buf)
    }
}
impl Write for DeadlineSocket {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.socket.set_write_timeout(Some(self.remaining()?))?;
        self.socket.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.remaining()?;
        self.socket.flush()
    }
}
enum Channel {
    Client(rustls::StreamOwned<ClientConnection, DeadlineSocket>),
    Server(rustls::StreamOwned<ServerConnection, DeadlineSocket>),
}
impl Channel {
    fn reset(&mut self, timeout: Duration) {
        match self {
            Self::Client(s) => s.sock.reset(timeout),
            Self::Server(s) => s.sock.reset(timeout),
        }
    }
    fn address(&self) -> std::io::Result<SocketAddr> {
        match self {
            Self::Client(s) => s.sock.socket.local_addr(),
            Self::Server(s) => s.sock.socket.local_addr(),
        }
    }
    fn close(&mut self, timeout: Duration) {
        self.reset(timeout);
        match self {
            Self::Client(s) => s.conn.send_close_notify(),
            Self::Server(s) => s.conn.send_close_notify(),
        }
        let _ = self.flush();
    }
}
impl Read for Channel {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Client(s) => s.read(buf),
            Self::Server(s) => s.read(buf),
        }
    }
}
impl Write for Channel {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Client(s) => s.write(buf),
            Self::Server(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Client(s) => s.flush(),
            Self::Server(s) => s.flush(),
        }
    }
}
enum Handle {
    Listener(TcpListener, Arc<ServerConfig>),
    Channel(Box<Channel>),
}

/// Owned by one interpreter/native execution; sockets close on drop, including failure.
pub struct Network {
    handles: BTreeMap<String, Handle>,
    allowed: BTreeSet<SocketAddr>,
    timeout: Duration,
    created: u64,
}
impl Default for Network {
    fn default() -> Self {
        Self {
            handles: BTreeMap::new(),
            allowed: BTreeSet::new(),
            timeout: Duration::from_secs(30),
            created: 0,
        }
    }
}
impl std::fmt::Debug for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Network")
            .field("open_handles", &self.handles.len())
            .finish_non_exhaustive()
    }
}
fn path(base: &Path, name: &str) -> std::path::PathBuf {
    base.join(name)
}
fn pem(path: &Path, secret: bool) -> Result<Zeroizing<Vec<u8>>> {
    let file = crate::crypto_cli::open_regular(path, MAX_PEM).map_err(io_error)?;
    #[cfg(unix)]
    if secret {
        use std::os::unix::fs::PermissionsExt;
        if file.metadata().map_err(io_error)?.permissions().mode() & 0o077 != 0 {
            return Err(error(
                "TLS private keys must have owner-only permissions (chmod 600).",
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = secret;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_PEM + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > MAX_PEM as usize {
        return Err(error("TLS credentials exceed 64 KiB."));
    }
    Ok(bytes)
}
fn certs(base: &Path, name: &str) -> Result<Vec<CertificateDer<'static>>> {
    let data = pem(&path(base, name), false)?;
    let certs = CertificateDer::pem_slice_iter(&data)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(io_error)?;
    if certs.is_empty() || certs.len() > 16 {
        return Err(error("Supply 1–16 PEM certificates."));
    }
    Ok(certs)
}
fn key(base: &Path, name: &str) -> Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_slice(&pem(&path(base, name), true)?).map_err(io_error)
}
fn roots(base: &Path, name: &str) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();
    for cert in certs(base, name)? {
        roots.add(cert).map_err(io_error)?;
    }
    Ok(roots)
}
fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

impl Network {
    fn permitted(&self, addr: SocketAddr) -> Result<()> {
        if addr.ip().is_loopback() || self.allowed.contains(&addr) {
            Ok(())
        } else {
            Err(error(
                "Non-loopback networking requires net_allow() for that exact numeric IP:port in this execution.",
            ))
        }
    }
    fn capacity(&self) -> Result<()> {
        if self.handles.len() >= MAX_HANDLES || self.created >= MAX_CREATED {
            Err(error(
                "Network handle budget exceeded (32 open; 1024 created).",
            ))
        } else {
            Ok(())
        }
    }
    fn insert(&mut self, handle: Handle) -> Result<Value> {
        self.created += 1;
        let mut nonce = [0u8; 16];
        provider()
            .secure_random
            .fill(&mut nonce)
            .map_err(io_error)?;
        let suffix = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let name = format!("goblin-net-{}-{suffix}", self.created);
        self.handles.insert(name.clone(), handle);
        Ok(Value::Text(name))
    }
    pub fn call(&mut self, base: &Path, name: &str, args: &[Value]) -> Result<Value> {
        arity(name, args.len())?;
        if name == "net_timeout" {
            let q = args[0].quantity(name)?;
            if q.dimension != crate::quantity::DIMENSIONLESS
                || !q.value_si.is_finite()
                || q.value_si.fract() != 0.0
                || !(1.0..=60_000.0).contains(&q.value_si)
            {
                return Err(error(
                    "net_timeout() expects an integer number of milliseconds, 1–60000.",
                ));
            }
            self.timeout = Duration::from_millis(q.value_si as u64);
            return Ok(Value::Bool(true));
        }
        let text = args
            .iter()
            .map(|v| v.text(name))
            .collect::<Result<Vec<_>>>()?;
        match name {
            "net_demo_credentials" => {
                demo_credentials(&path(base, text[0]))?;
                Ok(Value::Bool(true))
            }
            "net_pair_credentials" => {
                pair_credentials(&path(base, text[0]), text[1])?;
                Ok(Value::Bool(true))
            }
            "net_allow" => {
                let addr = address(text[0])?;
                if addr.port() == 0 || self.allowed.len() >= 64 {
                    return Err(error(
                        "Allow at most 64 exact endpoints with a nonzero port.",
                    ));
                }
                self.allowed.insert(addr);
                Ok(Value::Bool(true))
            }
            "net_listen" => {
                self.capacity()?;
                let addr = address(text[0])?;
                self.permitted(addr)?;
                let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
                    Arc::new(roots(base, text[3])?),
                    provider(),
                )
                .build()
                .map_err(io_error)?;
                let mut config = ServerConfig::builder_with_provider(provider())
                    .with_protocol_versions(&[&rustls::version::TLS13])
                    .map_err(io_error)?
                    .with_client_cert_verifier(verifier)
                    .with_single_cert(certs(base, text[1])?, key(base, text[2])?)
                    .map_err(io_error)?;
                config.alpn_protocols = vec![ALPN.to_vec()];
                config.send_tls13_tickets = 0;
                let listener = TcpListener::bind(addr).map_err(io_error)?;
                listener.set_nonblocking(true).map_err(io_error)?;
                self.insert(Handle::Listener(listener, Arc::new(config)))
            }
            "net_connect" => {
                self.capacity()?;
                let addr = address(text[0])?;
                self.permitted(addr)?;
                if addr.port() == 0 || text[1].len() > 253 {
                    return Err(error("Supply a nonzero port and certificate server name."));
                }
                let server_name = ServerName::try_from(text[1].to_string()).map_err(io_error)?;
                let mut config = ClientConfig::builder_with_provider(provider())
                    .with_protocol_versions(&[&rustls::version::TLS13])
                    .map_err(io_error)?
                    .with_root_certificates(roots(base, text[2])?)
                    .with_client_auth_cert(certs(base, text[3])?, key(base, text[4])?)
                    .map_err(io_error)?;
                config.alpn_protocols = vec![ALPN.to_vec()];
                config.resumption = rustls::client::Resumption::disabled();
                let deadline = Instant::now() + self.timeout;
                let socket = TcpStream::connect_timeout(&addr, self.timeout).map_err(io_error)?;
                socket.set_nodelay(true).map_err(io_error)?;
                let mut stream = rustls::StreamOwned::new(
                    ClientConnection::new(Arc::new(config), server_name).map_err(io_error)?,
                    DeadlineSocket { socket, deadline },
                );
                while stream.conn.is_handshaking() {
                    stream
                        .conn
                        .complete_io(&mut stream.sock)
                        .map_err(io_error)?;
                }
                if stream.conn.alpn_protocol() != Some(ALPN) {
                    return Err(error("Peer did not negotiate goblin-message/1."));
                }
                self.insert(Handle::Channel(Box::new(Channel::Client(stream))))
            }
            "net_accept" => {
                self.capacity()?;
                let Some(Handle::Listener(listener, config)) = self.handles.get(text[0]) else {
                    return Err(error("Expected an open listener handle."));
                };
                let deadline = Instant::now() + self.timeout;
                let socket = loop {
                    if Instant::now() >= deadline {
                        return Err(error("Network accept deadline reached."));
                    }
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2))
                        }
                        Err(e) => return Err(io_error(e)),
                    }
                };
                socket.set_nonblocking(false).map_err(io_error)?;
                socket.set_nodelay(true).map_err(io_error)?;
                let mut stream = rustls::StreamOwned::new(
                    ServerConnection::new(config.clone()).map_err(io_error)?,
                    DeadlineSocket { socket, deadline },
                );
                while stream.conn.is_handshaking() {
                    stream
                        .conn
                        .complete_io(&mut stream.sock)
                        .map_err(io_error)?;
                }
                if stream.conn.alpn_protocol() != Some(ALPN) {
                    return Err(error("Peer did not negotiate goblin-message/1."));
                }
                self.insert(Handle::Channel(Box::new(Channel::Server(stream))))
            }
            "net_send" | "net_receive" => {
                if name == "net_send" && text[1].len() > MAX_MESSAGE {
                    return Err(error("Message exceeds 1 MiB."));
                }
                let Some(Handle::Channel(mut channel)) = self.handles.remove(text[0]) else {
                    return Err(error("Expected an open connection handle."));
                };
                channel.reset(self.timeout);
                // Failure retires the connection: never reuse partially read/written framing.
                let result = if name == "net_send" {
                    channel
                        .write_all(&(text[1].len() as u32).to_be_bytes())
                        .and_then(|_| channel.write_all(text[1].as_bytes()))
                        .and_then(|_| channel.flush())
                        .map_err(io_error)
                        .map(|_| Value::Bool(true))
                } else {
                    receive(&mut channel)
                };
                if result.is_ok() {
                    self.handles
                        .insert(text[0].into(), Handle::Channel(channel));
                }
                result
            }
            "net_close" => {
                let mut handle = self
                    .handles
                    .remove(text[0])
                    .ok_or_else(|| error("Handle is closed or belongs to another execution."))?;
                if let Handle::Channel(channel) = &mut handle {
                    channel.close(self.timeout);
                }
                Ok(Value::Bool(true))
            }
            "net_peer_sha256" => {
                let Some(Handle::Channel(channel)) = self.handles.get(text[0]) else {
                    return Err(error("Expected an authenticated connection handle."));
                };
                let certificates = match channel.as_ref() {
                    Channel::Client(s) => s.conn.peer_certificates(),
                    Channel::Server(s) => s.conn.peer_certificates(),
                };
                let leaf = certificates
                    .and_then(|c| c.first())
                    .ok_or_else(|| error("Peer certificate is unavailable."))?;
                Ok(Value::Text(crate::hashing::sha256_bytes(leaf.as_ref())))
            }
            "net_address" => {
                let addr = match self.handles.get(text[0]) {
                    Some(Handle::Listener(listener, _)) => listener.local_addr(),
                    Some(Handle::Channel(channel)) => channel.address(),
                    None => return Err(error("Expected an open network handle.")),
                }
                .map_err(io_error)?;
                Ok(Value::Text(addr.to_string()))
            }
            _ => Err(error("Unknown network function.")),
        }
    }
}
fn receive(input: &mut impl Read) -> Result<Value> {
    let mut header = [0u8; 4];
    input.read_exact(&mut header).map_err(io_error)?;
    let size = u32::from_be_bytes(header) as usize;
    if size > MAX_MESSAGE {
        return Err(error("Remote message exceeds 1 MiB; connection retired."));
    }
    let mut bytes = vec![0; size];
    input.read_exact(&mut bytes).map_err(io_error)?;
    String::from_utf8(bytes).map(Value::Text).map_err(io_error)
}

/// Local learning credentials only. Each invocation creates a fresh CA and distinct leaf keys.
/// The CA signing key is not saved. Never package shared demo private keys.
pub fn demo_credentials(directory: &Path) -> Result<()> {
    let local_names = vec!["localhost".into(), "127.0.0.1".into(), "::1".into()];
    let files = credential_files(local_names.clone(), local_names)?;
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(directory).map_err(io_error)?;
    for (name, text) in files {
        crate::recipient_crypto::save_new(&directory.join(name), text.as_bytes())?;
    }
    Ok(())
}

fn credential_files(
    server_names: Vec<String>,
    client_names: Vec<String>,
) -> Result<Vec<(&'static str, Zeroizing<String>)>> {
    use rcgen::{
        BasicConstraints, CertificateParams, CertifiedIssuer, DnType, ExtendedKeyUsagePurpose,
        IsCa, KeyPair, KeyUsagePurpose,
    };
    let now = chrono::Utc::now().timestamp();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).map_err(io_error)?;
    // Use the existing time crate's inferred OffsetDateTime arithmetic, relative to rcgen epoch.
    let epoch = rcgen::date_time_ymd(1970, 1, 1);
    ca_params.not_before =
        epoch + std::time::Duration::from_secs(now.saturating_sub(300).max(0) as u64);
    ca_params.not_after =
        epoch + std::time::Duration::from_secs(now.saturating_add(90 * 86400).max(0) as u64);
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "Goblin++ local example CA");
    let ca_key = KeyPair::generate().map_err(io_error)?;
    let ca = CertifiedIssuer::self_signed(ca_params.clone(), ca_key).map_err(io_error)?;
    let mut files = vec![("ca.pem", Zeroizing::new(ca.pem()))];
    for (name, usage) in [
        ("server", ExtendedKeyUsagePurpose::ServerAuth),
        ("client", ExtendedKeyUsagePurpose::ClientAuth),
    ] {
        let mut params = CertificateParams::new(if name == "server" {
            server_names.clone()
        } else {
            client_names.clone()
        })
        .map_err(io_error)?;
        params.not_before = ca_params.not_before;
        params.not_after = ca_params.not_after;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![usage];
        params
            .distinguished_name
            .push(DnType::CommonName, format!("Goblin++ local {name}"));
        let key = KeyPair::generate().map_err(io_error)?;
        let certificate = params.signed_by(&key, &ca).map_err(io_error)?;
        files.push((
            if name == "server" {
                "server.pem"
            } else {
                "client.pem"
            },
            Zeroizing::new(certificate.pem()),
        ));
        files.push((
            if name == "server" {
                "server-key.pem"
            } else {
                "client-key.pem"
            },
            Zeroizing::new(key.serialize_pem()),
        ));
    }
    Ok(files)
}

/// Short-lived, dedicated learning pair for an explicit private IPv4 address.
/// The setup machine is trusted: it generates both identities. No CA signing key is saved.
pub fn pair_credentials(directory: &Path, server_ip: &str) -> Result<()> {
    let ip: std::net::Ipv4Addr = server_ip
        .parse()
        .map_err(|_| error("Supply the server's private LAN IPv4 address, without a port."))?;
    if !(ip.is_private() || ip.is_loopback()) || ip.octets()[0] == 0 {
        return Err(error(
            "LAN pair setup permits RFC1918 private IPv4 addresses, or loopback for local tests.",
        ));
    }
    let files = credential_files(vec![ip.to_string()], Vec::new())?;
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(directory).map_err(io_error)?;
    let ca = files
        .iter()
        .find(|(name, _)| *name == "ca.pem")
        .unwrap()
        .1
        .as_bytes();
    let fingerprint = crate::hashing::sha256_bytes(ca);
    for role in ["server", "client"] {
        let folder = directory.join(role);
        builder.create(&folder).map_err(io_error)?;
        for (name, text) in &files {
            if *name == "ca.pem" || name.starts_with(role) {
                crate::recipient_crypto::save_new(&folder.join(name), text.as_bytes())?;
            }
        }
        crate::recipient_crypto::save_new(
            &folder.join("CA_SHA256.txt"),
            format!("{fingerprint}  ca.pem\n").as_bytes(),
        )?;
        crate::recipient_crypto::save_new(
            &folder.join("SERVER_IP.txt"),
            ip.to_string().as_bytes(),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_limits_utf8_and_truncation_fail_closed() {
        for bytes in [
            u32::MAX.to_be_bytes().to_vec(),
            vec![0, 0, 0, 2, b'x'],
            vec![0, 0, 0, 1, 255],
        ] {
            assert!(receive(&mut bytes.as_slice()).is_err());
        }
        assert_eq!(
            receive(&mut [0u8; 4].as_slice()).unwrap(),
            Value::Text(String::new())
        );
    }

    #[test]
    fn missing_client_certificate_is_refused_before_message_delivery() {
        let d = tempfile::tempdir().unwrap();
        let directory = d.path().join("credentials");
        demo_credentials(&directory).unwrap();
        let mut server = Network::default();
        let text = |s: &str| Value::Text(s.into());
        let handle = server
            .call(
                &directory,
                "net_listen",
                &[
                    text("127.0.0.1:0"),
                    text("server.pem"),
                    text("server-key.pem"),
                    text("ca.pem"),
                ],
            )
            .unwrap();
        let addr = server
            .call(&directory, "net_address", std::slice::from_ref(&handle))
            .unwrap()
            .render();
        let client_roots = roots(&directory, "ca.pem").unwrap();
        let t =
            std::thread::spawn(move || server.call(&directory, "net_accept", &[handle]).is_err());
        let mut config = ClientConfig::builder_with_provider(provider())
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_root_certificates(client_roots)
            .with_no_client_auth();
        config.alpn_protocols = vec![ALPN.to_vec()];
        let socket = TcpStream::connect(addr).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut stream = rustls::StreamOwned::new(
            ClientConnection::new(Arc::new(config), ServerName::try_from("localhost").unwrap())
                .unwrap(),
            socket,
        );
        let _ = stream.write_all(b"must not arrive");
        let _ = stream.flush();
        assert!(t.join().unwrap());
    }

    #[test]
    fn plaintext_peer_cannot_stall_beyond_total_handshake_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let t = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            // TLS record announcing a body that never completes. Each fragment
            // arrives before an inactivity timeout, but not the total deadline.
            let _ = s.write_all(&[22, 3, 3, 0, 100]);
            for _ in 0..15 {
                std::thread::sleep(Duration::from_millis(10));
                if s.write_all(&[0]).is_err() {
                    break;
                }
            }
        });
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("credentials");
        demo_credentials(&dir).unwrap();
        let mut client = Network {
            timeout: Duration::from_millis(30),
            ..Network::default()
        };
        let args = [
            addr.as_str(),
            "localhost",
            "ca.pem",
            "client.pem",
            "client-key.pem",
        ]
        .map(|s| Value::Text(s.into()));
        let start = Instant::now();
        assert!(client.call(&dir, "net_connect", &args).is_err());
        assert!(start.elapsed() < Duration::from_millis(150));
        t.join().unwrap();
    }

    #[test]
    fn expired_server_and_tls12_only_server_are_refused() {
        use rcgen::{BasicConstraints, CertificateParams, CertifiedIssuer, IsCa, KeyPair};
        for expired in [true, false] {
            let mut ca_params = CertificateParams::new(vec![]).unwrap();
            ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
            let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();
            let mut leaf = CertificateParams::new(vec!["localhost".into()]).unwrap();
            if expired {
                leaf.not_before = rcgen::date_time_ymd(2000, 1, 1);
                leaf.not_after = rcgen::date_time_ymd(2001, 1, 1);
            }
            let key = KeyPair::generate().unwrap();
            let cert = leaf.signed_by(&key, &ca).unwrap();
            let versions: &[&rustls::SupportedProtocolVersion] = if expired {
                &[&rustls::version::TLS13]
            } else {
                &[&rustls::version::TLS12]
            };
            let config = ServerConfig::builder_with_provider(provider())
                .with_protocol_versions(versions)
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(
                    vec![cert.der().clone()],
                    rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
                )
                .unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap().to_string();
            let t = std::thread::spawn(move || {
                let (socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut stream = rustls::StreamOwned::new(
                    ServerConnection::new(Arc::new(config)).unwrap(),
                    socket,
                );
                stream.read_exact(&mut [0; 1]).is_err()
            });
            let d = tempfile::tempdir().unwrap();
            let dir = d.path().join("credentials");
            demo_credentials(&dir).unwrap();
            fs::write(dir.join("ca.pem"), ca.pem()).unwrap();
            let mut client = Network::default();
            let args = [
                addr.as_str(),
                "localhost",
                "ca.pem",
                "client.pem",
                "client-key.pem",
            ]
            .map(|s| Value::Text(s.into()));
            assert!(client.call(&dir, "net_connect", &args).is_err());
            assert!(t.join().unwrap());
        }
    }
}
