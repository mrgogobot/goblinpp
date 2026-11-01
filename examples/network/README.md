# Simple Goblin++ client and server

Use Goblin++ **0.1.0-alpha.44**. These programs exchange a greeting over TLS 1.3,
with both client and server certificates checked. No extra packages are needed
when using the bundled interpreter.

## 1. Create your own local credentials once

From the extracted alpha.44 bundle:

```sh
cd examples/network
goblin++ --version
goblin++ run setup.gbl
```

This creates `network-demo/` beside these source files. Keep its private keys
private; do not commit or share this directory. Setup refuses an existing
credential directory. For another exchange, reuse it and rerun server/client.

## 2. Start the server in terminal 1

```sh
goblin++ run server.gbl
```

The server waits up to 60 seconds. Its interpreted `print` output appears when
it finishes; silence while waiting is expected.

## 3. Run the client in terminal 2

Open a second terminal in the same `examples/network` folder:

```sh
goblin++ run client.gbl
```

Expected server output includes:

```text
Client said hello = true
RUN_STATUS=PASS
```

Expected client output includes:

```text
Server said hello = true
RUN_STATUS=PASS
```

The server accepts one client, replies and exits. Restart the server for another
exchange. Both use localhost port 7443; close the earlier networking demo first
if it is already using that port.

## IDE use

Open this folder as a writable project and run `setup.gbl` once. Use two IDE
terminal tabs, or separate IDE windows, for server and client. The main Goblin++
IDE Run panel manages one process at a time. Select the bundled alpha.44 engine
if an older executable path is saved.

## What is protected?

TLS protects the connection between these two programs. These basic examples
compare known greetings rather than displaying arbitrary untrusted network text.
`GO_PRIVATE` keeps messages and keys outside scientific receipts; it is not an
OS sandbox or secure erasure.

For recipient-encrypted messages through a relay, use the original examples
one folder above: `network_setup.gbl`, `network_server.gbl`,
`network_client.gbl` and `network_relay.gbl`. Full details and limits are in
[NETWORKING.md](../../docs/NETWORKING.md).
