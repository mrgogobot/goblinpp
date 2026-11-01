Alpha.45 adds [the two-computer LAN starter](../examples/network/lan/README.md)
and `net_pair_credentials(new_directory, server_ipv4)`. Existing alpha.44 APIs
below remain supported.

# Secure networking and recipient encryption — alpha.44

This first library exchanges bounded UTF-8 messages over TLS 1.3. Both peers must
present certificates rooted in explicitly supplied CA files. Separate recipient
message encryption keeps the plaintext away from a forwarding relay.

The implementation uses Rustls 0.23.46 (ring provider) and age 0.11.5, already
present in the preceding lockfile. Cryptographic keys and ephemeral material use
library/OS cryptographic randomness, never the scientific PCG stream. The Rust
compiler remains 1.92.0. This is a tested local alpha, not an independent security
certification, audited messenger, or general-purpose production server framework.

## Start here: two terminals

Extract the local bundle and install its runtime:

```sh
./install.sh --prefix "$HOME/.local"
rehash                         # zsh; use hash -r in bash
# goblin++ --version must say 0.1.0-alpha.44
cd examples
goblin++ run network_setup.gbl
```

Setup creates fresh `network-demo/` credentials. It refuses an existing directory
or output file. Keep that directory private and out of Git, backups shared with
others, release packages and AI context. The source checkout ignores the example
directory and the local packager excludes directories named `network-demo`.
Copies outside this checkout need their own `.gitignore` and storage policy.
There are no shared/demo private keys in the distributed package.

In terminal 1:

```sh
goblin++ run network_server.gbl
```

Within 60 seconds, in terminal 2, from the same `examples` directory:

```sh
goblin++ run network_client.gbl
```

The server decrypts the message, reports its character count, replies, then exits.
The client displays the reply as Base64 to avoid rendering terminal controls from
untrusted network text. Base64 is only display encoding. The server does not print
the message itself; `message` holds the plaintext for your program to use.
The library is synchronous. Run a fresh server for another client; this example
handles one connection. Output from interpreted `print` is collected until the
program finishes, so silence while waiting for a client is expected.

These are ordinary `.gbl` programs: open them in VS Code, CLion, PyCharm,
RustRover or Goblin++ IDE and use Run with the alpha.44 executable selected.
Use separate terminals or IDE windows for the server and client. An IDE's single
Run control may replace its previous process. Code completion/help is included
in VS Code 0.1.35 and JetBrains 0.3.25. Native compilation is supported too:

```sh
goblin++ run network_server.gbl --compile
# In another terminal, allowing time for the server compilation:
goblin++ run network_client.gbl --compile
```

Private native compilation deliberately stays offline. On a fresh development
machine, first run `cargo fetch --locked` in the matching public engine checkout.
It prepares dependencies before private execution; it does not upload source.

## A minimal exchange in code

Both programs begin with `GO_PRIVATE`. On the server:

```goblin
GO_PRIVATE
net_timeout(60000)
listener = net_listen("127.0.0.1:7443", "server.pem", "server-key.pem", "ca.pem")
client = net_accept(listener)
message = net_receive(client)
net_send(client, "Message received.")
net_close(client)
net_close(listener)
```

On the client:

```goblin
GO_PRIVATE
server = net_connect("127.0.0.1:7443", "localhost", "ca.pem", "client.pem", "client-key.pem")
net_send(server, "Hello!")
reply = net_receive(server)
net_close(server)
```

The five `net_connect` arguments are address, expected certificate server name,
CA trust file, client certificate and client private key. The four `net_listen`
arguments are bind address, server certificate, server private key and client CA
trust file. Relative file paths resolve from the source directory under `run`;
standalone compiled programs use their working directory.

## Recipient-only encryption

```goblin
GO_PRIVATE
# Recipient creates these once, retaining recipient.key privately.
public_recipient = e2e_keygen("recipient.key", "recipient.txt")
```

Only distribute `recipient.txt`. Authenticate that public key independently
(for example by comparing it through an already trusted channel). Replacing a
recipient key with an attacker's key defeats recipient confidentiality.

The sender needs only the public key:

```goblin
GO_PRIVATE
recipient = read_text("recipient.txt")
envelope = e2e_encrypt("Private experiment discussion", recipient)
# Send envelope with net_send(), or pass it through a relay.
```

The recipient alone needs the private key:

```goblin
GO_PRIVATE
# envelope would normally come from net_receive().
# plaintext = e2e_decrypt(envelope, "recipient.key")
```

`e2e_encrypt` uses standard binary age v1 with one X25519 recipient, wrapped in
canonical padded RFC 4648 Base64 for transport as a Goblin++ string. This outer
Base64 differs from the age format's internal unpadded encoding and from age
ASCII armor. For other age implementations, Base64-decode the whole returned
string to obtain an ordinary binary `.age` file. Existing passphrase-based
`encrypt_file`/`decrypt_file` functions remain separate and unchanged.

`e2e_keygen(private_file[, public_file])` returns the public recipient key;
`e2e_public_key(private_file)` derives it again. All output files are new, with
Unix mode 0600; no existing file or symlink is replaced. A two-file key generation
is not a transaction: if the second write fails, the first private key remains.
Store private keys securely. These key files are not themselves encrypted at rest;
Unix permissions do not stop an administrator or compromised user account.

## Try a relay

Start `network_server.gbl`, then `network_relay.gbl` in a second terminal, then:

```sh
goblin++ run network_client.gbl -- 127.0.0.1:7444
```

The relay forwards the encrypted envelope and the reply. It never opens the
recipient key. The single-machine learning directory contains all roles' keys;
that is not physical isolation. On separate machines, give a relay only its own
TLS identity and required public CA material, never the recipient private key.
Our integration check also runs a relay from a separate directory with no
recipient private key.

## API and limits

| Function | Result / rule |
|---|---|
| `net_demo_credentials(new_directory)` | Fresh localhost learning CA and separate server/client keys; 90-day validity; CA signing key is not saved. Refuses existing directory. |
| `net_allow("IP:port")` | Explicitly permit that exact numeric non-loopback endpoint for this execution; at most 64. |
| `net_timeout(milliseconds)` | Integer 1–60000; default 30000; absolute deadline per accept/handshake/send/receive operation, including partial traffic. |
| `net_listen(address, certificate, private_key, ca)` | Listener handle; TLS 1.3, mandatory client certificate. Loopback port 0 may select an ephemeral port. |
| `net_connect(address, server_name, ca, certificate, private_key)` | Connection handle; verifies server chain, validity and name; sends a client certificate. |
| `net_accept(listener)` | One authenticated connection handle. |
| `net_send(connection, text)` | Send one UTF-8 message, at most 1 MiB. A four-byte big-endian length prefix is inside TLS. Success is not proof of application processing. |
| `net_receive(connection)` | One UTF-8 message; no truncation or undecodable bytes are returned as success. |
| `net_peer_sha256(connection)` | Lowercase SHA-256 of the authenticated peer leaf certificate DER, for explicit application authorization/pinning. Obtain expected fingerprints independently. |
| `net_address(handle)` | Local numeric socket address. |
| `net_close(handle)` | Close connection/listener; invalid or already closed handles refuse. |
| `e2e_keygen(private_file[, public_file])` | New X25519 identity; public recipient key returned. |
| `e2e_public_key(private_file)` | Public key derived from a private identity file. |
| `e2e_encrypt(text, public_recipient)` | Age ciphertext envelope; plaintext limit 512 KiB. |
| `e2e_decrypt(envelope, private_file)` | Fully authenticated UTF-8 plaintext; envelope at most 1 MiB and plaintext at most 512 KiB. Password/KDF messages are refused. |

There are at most 32 open handles and 1024 created handles per execution. Sockets
close when their owning execution drops, including on errors. A receive/transport
failure retires its connection, preventing reuse after partial framing. Credentials
are bounded to 64 KiB (1–16 certificates); identity keys to 1 KiB. Unix private
key files require no group/other permission bits and direct symlinks are refused.
Ancestor-directory protection is the user's/OS's responsibility.

## Local versus remote policy

Only loopback endpoints work by default. To connect elsewhere, explicitly permit
the exact numeric destination, then use the correct certificate server name:

```goblin
GO_PRIVATE
net_allow("192.0.2.10:7443")
# Documentation-only address: replace it with your authorized server's address.
# peer = net_connect("192.0.2.10:7443", "science.example.org", "ca.pem", "client.pem", "client-key.pem")
```

A non-loopback listener also requires permission for its exact local bind address
and port. Wildcard, broadcast and multicast addresses are refused. Numeric IPs
avoid hidden DNS resolution/rebinding; there are no environment proxies, redirects,
HTTP parsing, shell commands, insecure options or plaintext fallback in this API.
The existing bounded public HTTPS `goblin++ fetch` remains its own acquisition path.

Use separately provisioned, current credentials for real deployments; demo leaf
certificates are for localhost. Mutual TLS proves membership in the supplied CA's
trust domain. It does not decide which authenticated clients may perform which
actions. Compare independently provisioned peer fingerprints or implement your own
application authorization before processing requests. Automatic certificate
revocation/rotation, CRL/OCSP service integration, concurrent connection scheduling,
rate limiting across clients and a production service lifecycle are not included.

## Precise trust boundaries

- TLS protects each connection. A relay terminates TLS on both sides. Recipient
  encryption protects the message through that relay, but not against a compromised
  sender/recipient device or stolen private key.
- age authenticates ciphertext integrity, not the sender's identity. Anyone with
  the public recipient key can encrypt a message. It adds no message signature,
  application replay/deduplication protection, forward-secrecy ratchet or
  post-quantum guarantee. Use an established messaging protocol if you require them.
- TLS early data and session resumption are disabled here. TLS 1.2-only peers,
  missing/untrusted/expired certificates and wrong server names refuse.
- Receipt integrity and confidentiality are separate. `GO_PRIVATE` deliberately
  creates no scientific receipt, snapshots, ledger, seals or final-state capture;
  scientific profiles cannot call these functions. It is not an OS sandbox,
  encrypted-memory system or secure-erasure guarantee. Explicit print/save,
  hardcoded strings, command-line arguments and AI submissions can expose secrets.
- Received text is untrusted. Escaping or safe encoding is necessary before
  displaying arbitrary terminal content. Do not execute messages as code.

References: [TLS 1.3](https://www.rfc-editor.org/rfc/rfc8446),
[Rustls](https://github.com/rustls/rustls),
[age format](https://c2sp.org/age), [age Rust implementation](https://github.com/str4d/rage).

Optional Goblin++ merchandise: https://h4k3rl1f3.myspreadshop.co.uk
