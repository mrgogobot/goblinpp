# Explicit scientific-file acquisition — local alpha.36 candidate

Status: implemented locally as Unix CLI commands, **not yet publicly released**.
This is the first GBL-026 stage. GBL-027 compression and GBL-028 integrated cube
handles remain queued. There is no `download()` language builtin, general socket
API, server/listener, upload, remote execution or implicit network import.

## Two deliberate steps: acquire, then use the local snapshot

Use a public HTTPS download URL and explicitly permit its lowercase hostname.
The examples contain placeholders: replace them with your actual public URL,
host, independently obtained hash and local paths; no example downloads data.

```sh
goblin++ fetch "https://archive.example.org/public/cube.fits" \
  --allow-host archive.example.org --sha256 EXPECTED_64_LOWERCASE_HEX_DIGITS \
  --output new-cube-acquisition --filename cube.fits
```

The output directory must not already exist, including after a failed attempt.
Use a new directory for an intentional retry; no automatic retry or resume is
performed. The downloader ignores remote filename suggestions. Output defaults
to `input.fits` but can contain any raw file type: the name/MIME type is not a
format or scientific-validity check. Nothing downloaded is automatically run,
opened in a browser, interpreted, decompressed or parsed as FITS.

If there is **no independently obtained SHA-256**, explicitly choose observed-only
recording instead. Do not invent an expected hash or call a server ETag trusted:

```sh
goblin++ fetch "https://archive.example.org/public/cube.fits" \
  --allow-host archive.example.org --record-only --output new-observed-cube
```

`COMPLETE_HASH_VERIFIED` means actual payload bytes match the caller's expected
hash. `COMPLETE_OBSERVED_ONLY` means the transfer completed and its observed
bytes/hash were recorded; it is **not** an externally anchored content match.
Neither establishes calibration, scientific reliability, authorship or permission
to retrieve the data. An expected hash needs its own trustworthy provenance.

Keep the printed `RECEIPT_CORE_SHA256` independently, then verify offline:

```sh
goblin++ verify-fetch new-cube-acquisition \
  --expected-receipt-sha256 ACTUAL_PRINTED_RECEIPT_CORE_SHA256
```

`verify-fetch` never contacts the URL. It checks the versioned policy, receipt
core, optional external receipt anchor, exact local byte counts and hashes,
safe paths and complete two-file inventory. Omitting the external anchor gives
self-consistency, not proof against an attacker replacing the entire bundle.
A verified failed acquisition remains `FAILED`; byte verification must not be
read as download success. Unlisted files or a postflight-failure marker refuse.

After a successful acquisition, use the printed absolute payload path:

```sh
goblin++ fits-info "/actual/path/new-cube-acquisition/cube.fits"
goblin++ run examples/downloaded_fits.gbl -- "/actual/path/new-cube-acquisition/cube.fits"
```

The example inspects HDU count only. Existing local-import rules and limits still
apply; it is not integrated spatial-cube extraction, WCS interpretation or a new
scientific analysis. A later `.gbl` run snapshots/hashes the local input through
its existing custody path; it does not automatically import the acquisition's
receipt as parent evidence. Keep/explicitly seal the acquisition anchor when
linking analyses manually. Acquisition receipts are distinct from standard run
receipts and do not automatically append to the project custody ledger.

## Network security contract

Policy: `goblin.public-https-acquisition.v1`.
Receipt schema: `goblin.acquisition-receipt.v1`.

- HTTPS GET only, port 443. An exact lowercase DNS host must be supplied with
  `--allow-host`; no wildcard permissions, IP literals, URL userinfo, fragments,
  control characters, backslashes or ambiguous authority encodings. International
  paths must be URL-encoded; host spelling must be explicit ASCII DNS spelling.
- The resolver hands the connector the **same vetted SocketAddr candidates**.
  There is no second hostname lookup after the address check. Every candidate
  that may be dialled must satisfy the public-routing policy; a mixed public/
  private candidate set is refused, not filtered into an ambiguous success.
  Loopback, private, link-local, metadata, documentation, multicast, reserved,
  mapped and transition/special-use ranges are conservatively excluded. This
  may refuse some unusual legitimate endpoints; it is not an exhaustive network
  classification service or a guarantee about OS routing.
- Rustls certificate/hostname verification and SNI stay enabled; OS/platform
  trust is used. There is **no insecure option** or user-supplied TLS-root or
  client-certificate option. Test-only local routing/certificates do not exist
  in production builds. The HTTP client's low-level resolver API is version-pinned.
- Redirects are refused, including same-host redirects. No redirect Location is
  visited or copied into a receipt. A CDN needing a new host requires a separate
  explicit request and host permission, not an automatic trust expansion.
- Proxy environment settings are overridden with no proxy. No cookies,
  credential headers, automatic authentication, request-body upload or Referer
  chain is provided. HTTP status must be 200, with no Content-Range. Conflicting
  framing is refused by the HTTP parser/profile.
- `Accept-Encoding: identity` is requested. Nonidentity Content-Encoding is
  refused; gzip/brotli/charset/JSON automatic-decoding features are disabled.
  HTTP chunk framing is removed normally, but scientific-file bytes are not
  decompressed or recoded. A raw `.fits.gz` file can be acquired; decoding it
  into FITS is a **separate, not-yet-implemented** scientific-input stage.
- Receipt/errors omit raw URL paths and queries and raw ETag/Last-Modified
  values; their hashes preserve comparison identities. This is redaction, not
  encryption. Do not put passwords, tokens or signed-URL secrets on a command
  line: shell history, process inspection and external diagnostic logging are
  outside this protection. This stage is intended for public unauthenticated
  downloads, not secret-bearing URLs.

Only origin and a full-URL hash are retained by default: the hash cannot recreate
the original URL. Reacquisition needs the original public URL retained separately;
offline analysis instead uses the preserved bytes. Fetch command-line syntax
errors intentionally omit supplied values too; use `fetch --help` for argument
details. This does not remove URLs from shell history or OS process inspection.

The dependency is maintained `ureq` with rustls/platform verification, not custom
cryptography. A build-lock hash identifies the actual locked dependency graph;
client/policy identity is retained in the receipt. Rust remains 1.92.0. See the
primary [ureq TLS/root documentation](https://docs.rs/ureq/3.4.2/ureq/#tls-https)
and [resolver contract](https://docs.rs/ureq/3.4.2/ureq/unversioned/resolver/trait.Resolver.html).

## Bounds, storage and failure

| Bound | Default / maximum |
|---|---|
| Accepted payload bytes | 256 MiB default; `--max-bytes` 1 byte–1 GiB |
| Transfer deadline | 300 s default; `--timeout-seconds` 1–3,600 s |
| DNS lookup | At most 5 s or the shorter transfer deadline |
| Connect/TLS | At most 10 s or the shorter transfer deadline |
| Response header wait | At most 30 s or the shorter transfer deadline |
| Response headers | 16 KiB |
| Receipt read | 64 KiB |
| Application copy/hash buffers | 64 KiB each; not a process-wide memory claim |
| HTTP application attempts / redirects | One / zero |

Actual accepted payload bytes are counted even without a size header; claimed
sizes are not trusted as the sole guard. An unknown-length oversize body uses
at most one extra plaintext byte to detect overflow, not to enlarge the saved
payload. These are payload bounds, **not exact wire-traffic, OS-buffer, filesystem
overhead, whole-process RAM or total-system resource caps**. TLS/client/parser
buffers and network headers add overhead. Hashing/postflight/local filesystem
work can take time beyond the network transfer deadline. An OS DNS worker may
finish after its caller times out; this is not forced cancellation of system I/O.

Output uses a new mode-0700 Unix directory, exclusively created mode-0600 files
and descriptor-relative no-follow operations. Partial payloads remain
`payload.part`. Successful payloads are published without replacing an existing
name and made mode 0400. The owner can change permissions later: this is not
write-once storage. Existing paths, symlink components, special files and
multiply linked files are refused. Use canonical paths rather than macOS
`/tmp`/`/var` symlink aliases. Ordinary I/O/space failures can prevent a complete
receipt from being written; the command must not claim success in that case.

A preserved transfer failure records a fixed redacted failure reason, partial
byte count and hash, not a valid complete input. Validation before output
creation leaves no acquisition bundle. Interrupted/killed processes may leave
incomplete directories; absence of a valid complete receipt is not success.
Receipt and payload are independently re-read before a successful CLI return.
Postflight failure adds a marker when possible; such a bundle cannot verify.

## What these protections do not establish

- No OS sandbox, firewall, network-wide egress control or protection from
  compromised DNS/trust/routing or privileged/hostile filesystems. DNS and OS
  trust/revocation services are outside the HTTP destination allowlist.
- No atomic forensic acquisition, signed authorship, certified timestamps,
  encrypted-at-rest storage, malicious-file certification or scientific result.
- No physical blinding. A future isolated worker must receive only authorised
  snapshots and lack access to source catalogues/network/inline-Rust escapes.
- No transparent `.gbl` network effects: checking, compiling, running local
  data operations and offline verification do not invoke this downloader.
  Authorized inline Rust retains its pre-existing full user authority.
- No general TCP/UDP/WebSocket API, credentials, LAN allowance, automatic retries,
  resumable transfers, 3-D compression, cube handles or a `COMPLETELY_MAD` profile.

## Acceptance and publication gates

The local regression suite uses controlled TLS fixtures, generated test
certificates and test-only resolution/routing. It never needs a live archive.
Positive exact binary/empty/8 MiB cases are paired with certificate-name,
untrusted-root and expiry failures; redirect/status/range/encoding/framing,
header/byte/deadline, external-hash, path/link/overwrite, policy/receipt/payload
tampering, explicit host/mode and offline-verification gates are covered.

Before public release, require the full existing suite, formatting and warning-
denying Clippy, macOS/Linux CI, current RustSec advisory review of the locked
dependencies, regenerated third-party notices and review of the network/privacy
contract. Tests and an advisory scan are not a formal security audit or a promise
of vulnerability-free software. Keep previously released archives unchanged.

Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk); no purchase
is required to use the language.
