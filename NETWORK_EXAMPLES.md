Two-computer LAN starter: see **examples/network/lan/QUICKSTART.md**.
Run the source examples directly with the supplied runtime.

# Start here: simple client and server

The simple examples are in **examples/network/**:

- **setup.gbl** — generate your own local TLS credentials once.
- **server.gbl** — receive one greeting and reply.
- **client.gbl** — send a greeting and receive the reply.
- **README.md** — full instructions and expected output.

Install the bundled alpha.44 runtime from this bundle's top folder:

```sh
./install.sh --prefix "$HOME/.local"
rehash                    # zsh; use hash -r for bash
cd examples/network
goblin++ --version       # must report 0.1.0-alpha.44
goblin++ run setup.gbl
```

In terminal 1, run `goblin++ run server.gbl`.
In terminal 2, from the same folder, run `goblin++ run client.gbl`.
The server waits for one client, then both programs exit.

These examples use mutually authenticated TLS 1.3 on localhost. The original
recipient-encryption/relay examples remain under `examples/network_*.gbl`.
See `docs/NETWORKING.md` for the complete API and security boundaries.
