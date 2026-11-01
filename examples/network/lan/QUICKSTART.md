# Run the LAN examples

Use alpha.45 on the setup/server computer. The client/server scripts use network
APIs available in alpha.44 or newer. These instructions run source examples; no
separate compiled client/server is needed.

## On the Mac used as the server

From the extracted runtime bundle, install and create credentials:

```sh
./install.sh --prefix "$HOME/.local"
rehash
cd examples/network/lan
goblin++ --version
goblin++ run setup.gbl
```

Enter the server Mac's PRIVATE LAN IPv4 address when prompted, such as
192.168.1.42. Use the actual address shown in macOS Network settings, not the
example address and not 127.0.0.1. Setup creates lan-pair/server and lan-pair/client.

Copy each script into its corresponding role folder:

```sh
cp network_server.gbl lan-pair/server/
cp network_client.gbl lan-pair/client/
```

Transfer ONLY the client folder to the other computer through a trusted channel,
such as a USB drive you control. Keep the server folder on the server. Confirm
the ca.pem file fingerprint with the trusted setup computer through a separate
trusted channel; CA_SHA256.txt arriving with a file is not independent proof.

Start the server from the same examples/network/lan folder:

```sh
goblin++ run lan-pair/server/network_server.gbl
```

It waits for one client for up to 60 seconds; restart it if it times out.

## On the client computer

Use an alpha.44-or-newer Goblin++ runtime. With this source bundle, install.sh
builds the Linux executable from locked source when run on Linux; Rust 1.92.0
must already be available. The supplied Mac binary does not run on Linux.

Copy the client folder onto the client's own local filesystem, open a terminal
inside that folder and run:

```sh
chmod 700 .
chmod 600 client-key.pem
goblin++ --version
goblin++ run network_client.gbl
```

Both programs should print NETWORK_EXCHANGE_VERIFIED=true and RUN_STATUS=PASS.
These expected greetings check this example, not arbitrary scientific validity.

## If it cannot connect

Check the server IP, both clocks, the matching CA, client runtime version and key
permissions. Both computers must be on a mutually reachable LAN; guest Wi-Fi may
block peer connections. The server must allow TCP 7443 from the intended client
through its normal firewall settings. Do not disable the firewall or certificate
verification, and do not expose the port to the Internet for this LAN test.

The setup machine initially sees both private keys; this is a short-lived learning
pair, not a managed production PKI. Credentials expire after 90 days. See README.md
for full trust boundaries and optional native/Linux build instructions. The local
exchange is tested; a physical two-computer test remains yours to perform.
