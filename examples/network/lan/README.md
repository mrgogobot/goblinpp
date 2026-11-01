For the simplest source-based workflow, start with [QUICKSTART.md](QUICKSTART.md).
Separate compiled client/server programs are optional; the full runtime bundle
contains everything needed to run the examples on macOS arm64. Linux requires
a compatible runtime or a source build as described below.

# Two-computer LAN starter — Goblin++ alpha.45

This starter provides `network_server` and `network_client` executables for
macOS arm64, plus their readable Goblin++ source. A compiled `network_setup`
creates one dedicated, short-lived pair of TLS identities. Existing alpha.44
networking remains available.

Both endpoints authenticate with TLS 1.3. The server's certificate includes its
actual private IPv4 address; the client verifies that address, certificate and
trust chain. Each computer receives only its own private key. No passwords or
certificate-bypass switches are needed for a connection.

## What you need

- Two computers connected to the same trusted home/lab network.
- The server's current private IPv4 address, for example `192.168.1.42`.
- A trusted way to transfer the client folder, such as a USB drive you control.
- Accurate system clocks. These learning credentials expire after 90 days.

The first helper supports RFC1918 IPv4 addresses (`10.*`, `172.16–31.*`,
`192.168.*`). Loopback is accepted for local tests. This is an IPv4 LAN starter;
IPv6 and Internet deployment are not part of these setup instructions.

Find the server's address in its network settings. On a Mac, Wi-Fi's address can
usually be displayed with `ipconfig getifaddr en0`; on Linux use `ip -4 addr`.
Choose the address on the interface shared with the client, not `127.0.0.1`, a
container interface or an unrelated VPN. Do not enter `0.0.0.0`.

## 1. Generate a dedicated pair on one trusted setup computer

In the standalone test kit's folder on your Mac:

```sh
./macos-arm64/network_setup
```

Enter the server's IPv4 address when prompted. Alternatively, with the new
runtime installed, from this source example folder run:

```sh
goblin++ run setup.gbl
```

The result is:

```text
lan-pair/
  server/  ca.pem, server.pem, server-key.pem, SERVER_IP.txt, CA_SHA256.txt
  client/  ca.pem, client.pem, client-key.pem, SERVER_IP.txt, CA_SHA256.txt
```

The signing key for the pair's private CA is not saved. Setup refuses an existing
output directory. This trusted setup computer initially sees BOTH private keys;
that is a provisioning tradeoff, not distributed key generation. For production,
use an established PKI/CSR workflow where each endpoint creates its own key and
where authorization, revocation and renewal are managed.

## 2. Put the correct program into each role folder

For two macOS arm64 computers, from the standalone kit:

```sh
cp macos-arm64/network_server lan-pair/server/
cp macos-arm64/network_client lan-pair/client/
```

Keep `lan-pair/server/` on the server computer. Transfer ONLY `lan-pair/client/`
to the client computer over your trusted channel. Do not give either peer the
other peer's private key or copy the entire `lan-pair` parent folder to it.

After copying to a Unix computer, protect the copied folder and its private key:

```sh
chmod 700 server       # or client, for that computer's role folder
chmod 600 server/server-key.pem  # client/client-key.pem on the client
```

Run these from the folder above `server/` or `client/`. Never work around a
private-key permission refusal by weakening the key's permissions.

Check the CA fingerprint from the original trusted setup computer through a
separate trusted channel. On macOS use `shasum -a 256 ca.pem`; on Linux use
`sha256sum ca.pem`, from inside each role folder. Both CA files must match the
original value. `CA_SHA256.txt` is a convenient copy, not an independent trust
anchor if it arrived through the same untrusted transfer as `ca.pem`.

## 3. Start the server

Open a terminal inside the server folder on the server computer:

```sh
./network_server
```

It reads `SERVER_IP.txt`, grants permission only for that exact local IP and port
7443, binds that specific interface and waits up to 60 seconds for one client.
It does not bind a wildcard interface. Restart it if it times out.

## 4. Start the client

Within 60 seconds, open a terminal inside the client folder on the other computer:

```sh
./network_client
```

Both should display:

```text
NETWORK_EXCHANGE_VERIFIED=true
RUN_STATUS=PASS
```

`NETWORK_EXCHANGE_VERIFIED` checks this example's expected greeting/reply.
`RUN_STATUS=PASS` by itself means the program ran without a machinery error; it
is not proof that an arbitrary application request was authorized or processed.
Each program handles one exchange and exits. Prepare both terminals before
starting, then restart the server for another test. Interpreted print output is
collected until completion; silence while accepting a client is expected.

## Linux / another CPU architecture

The supplied native executables are macOS arm64 only. Do not run them on Linux
or Intel Macs. The `.gbl` client/server sources use APIs already in alpha.44:
copy `network_client.gbl` or `network_server.gbl` into the appropriate role folder
and run it with a matching Goblin++ runtime, for example:

```sh
goblin++ run network_client.gbl
```

To build native versions on Linux using the source in the runtime bundle, retain
Rust 1.92.0, prepare public dependencies first, and compile the two sources:

```sh
# From the extracted runtime source bundle, with Rust/rustup already installed:
rustup run 1.92.0 cargo fetch --locked
rustup run 1.92.0 cargo build --release --locked
rustup run 1.92.0 ./target/release/goblinpp compile examples/network/lan/network_client.gbl --output network_client
rustup run 1.92.0 ./target/release/goblinpp compile examples/network/lan/network_server.gbl --output network_server
```

Copy each resulting program into its correct role folder. Do not copy the
`.native-build` compiler work directories or distribute private credentials.
If the retained toolchain is not installed, install that specific toolchain first;
this workflow does not require changing your default compiler version.

## Troubleshooting without weakening security

- Confirm both computers are on the same reachable LAN; guest Wi-Fi/client
  isolation may deliberately prevent peer-to-peer connections.
- Verify `SERVER_IP.txt` and the server's current address. If DHCP changed it,
  generate a NEW pair and redistribute the correct folders; editing the IP text
  file does not update the certificate's authenticated IP identity.
- If a firewall blocks the connection, configure only the intended server
  application/port 7443 from the intended client address, using your platform's
  normal firewall controls. This kit does not change firewall settings.
- Verify CA fingerprints, system clocks, key permissions and certificate validity.
  Wrong names, wrong trust, wrong/missing client certificates and tampering refuse.
- Never forward the port through your router or disable certificate verification
  just to make this LAN test succeed.

## Security scope

`GO_PRIVATE` creates no scientific run receipt, ledger or source snapshot.
TLS provides connection encryption and mutual certificate authentication; it does
not make an application correct, concurrent, rate-limited or safe against all
resource-exhaustion attacks. The starter has bounded messages/timeouts and a
single-client lifecycle, not a production service manager.

A trusted setup machine, correctly distributed trust and protected endpoints are
required. These short-lived pairing certificates are a learning convenience,
not a managed CA/revocation system. The existing `e2e_encrypt`/`e2e_decrypt` can
protect recipient messages through relays; age messages add no sender signatures,
application replay prevention or forward-secrecy ratchet.

Local automated tests exercise separate credential folders, certificate IP
matching, private-key permissions, explicit endpoint permission and compiled
client/server exchanges. An actual two-machine deployment still needs testing on
your own Mac/Linux network. No independent security audit or speed benchmark is
claimed; “faster” here means fewer steps to implement a checked connection.

Standards: [TLS 1.3](https://www.rfc-editor.org/rfc/rfc8446),
[X.509 certificates](https://www.rfc-editor.org/rfc/rfc5280),
[Rustls](https://github.com/rustls/rustls).

Optional Goblin++ merchandise: https://h4k3rl1f3.myspreadshop.co.uk
