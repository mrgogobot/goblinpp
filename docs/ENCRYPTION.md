# File protection and Base64 — alpha.43

Goblin++ now provides explicit **passphrase-based age v1 file encryption** through
Rust's `age` 0.11.5 implementation. It uses the published age format, scrypt
password derivation and authenticated ChaCha20-Poly1305 streaming. Goblin++ does
not implement a cipher, select nonces, or use its scientific PCG32 generator for
cryptography. Files interoperate with the reference age implementation.

This is a local alpha implementation, not an independent security certification.
The upstream Rust age crate itself labels pre-1.0 versions beta. This command is
**age encryption, not AES-256-GCM**. The earlier AES proposal has not been shipped
under a misleading name. There is no custom Goblin++ encrypted-file format.

## Everyday commands

Protect an existing file, using a hidden password prompt and confirmation:

```sh
goblin++ encrypt --input measurements.csv --output measurements.csv.age
# Equivalent requested shortcuts:
goblin++ -enc -of measurements.csv -nf measurements.csv.age
```

Recover its exact bytes into a **new** file:

```sh
goblin++ decrypt --input measurements.csv.age --output restored.csv
goblin++ -dec -of measurements.csv.age -nf restored.csv
```

`-nf` means new output file. `-of` means existing input file. `--enc` / `--dec`
also work. Explicit subcommands are preferable in scripts. Paths containing
spaces must be quoted. Options do not change program arguments after `run ... --`.

Encrypt new text without placing it in process arguments:

```sh
goblin++ encrypt --text-stdin --output note.age
```

Enter and confirm the hidden password. Then supply plaintext on standard input,
ending with EOF (Ctrl-D on Unix). **Ordinary terminal text entry is visible**;
for hidden text entry use the IDE's Encrypt New Text action, or redirect stdin
from an existing private file. Redirection still leaves that original file in
plaintext. Goblin++ does not delete it.

The user-requested literal form is supported:

```sh
# DEMONSTRATION ONLY: these literal values are not real secrets.
goblin++ -enc -key "example-not-a-real-password" -nf demonstration.age "here is a demonstration"
```

**Literal `-key` / `--key` passwords and literal plaintext can leak into shell
history, process inspection, terminal recordings, or external monitoring.**
Goblin++ warns about both and omits argument values from parser/error output,
but it cannot remove another program's copies. Use the hidden prompt instead.
Encryption requires at least 12 UTF-8 password bytes, at most 4096, no newline,
CR or NUL. Decryption accepts shorter historical passwords. Use a long, unique,
randomly generated passphrase; length alone does not imply adequate entropy.
There is no password recovery. Store the password separately from the file.

For controlled IPC, `--key-stdin` reads exactly the first UTF-8 line as the
password, removing its LF or CRLF terminator, preserving all other spaces.
If `--text-stdin` is also selected, remaining bytes are the plaintext. The IDEs
use this private pipe protocol. Do not put a secret in a shell `echo` command.
No environment-variable password interface, key persistence or key escrow is
provided. There are no directory/archive commands in this stage.

## Explicit screen display

```sh
# Display an ordinary UTF-8 file:
goblin++ read-file restored.csv
goblin++ -of restored.csv

# Authenticate the entire encrypted text before displaying it:
goblin++ -dec -of note.age --show
```

Screen display is explicit, limited to 1 MiB UTF-8, and escapes terminal controls
and bidi formatting controls while preserving line breaks and tabs. It is a
safe-to-render **preview**, not a lossless byte export. Binary/oversized inputs
refuse before displaying a prefix. Use decryption to a new file for exact bytes.
Screen history, screen sharing, IDE logs and clipboard use can expose displayed
plaintext. Preview decryption keeps plaintext in bounded memory, without a
plaintext temporary file; nothing is displayed before authenticated EOF.

## IDEs

Install VS Code **0.1.34** or JetBrains **0.3.24** and select/install the matching
alpha.43 executable. Updating the plugin alone does not update the executable.

- VS Code Command Palette: **Goblin++: Encrypt a File (age)**, **Encrypt New Text
  (age)**, **Decrypt a File (age)**, **Display a UTF-8 File**.
- JetBrains Tools → Goblin++: the same four actions. These work with ordinary
  files, not only `.gbl` programs. JetBrains asks for an absolute new output path.

Passwords and new secret text use masked dialogs. New text entry is limited to 1 MiB; use a file for larger content. Encryption confirms the
password. Neither password nor new text is passed in process arguments, saved
in run configurations/settings, or sent through the console input mechanism.
The plugins send them directly to the child's stdin pipe. File actions refuse in untrusted workspaces/projects; choose a trusted executable before entering a secret. Cancelling a prompt
starts no operation. Explicit file previews warn before displaying plaintext.
IDE/JavaScript/Java strings and OS pipe buffers cannot promise secure erasure;
there is no protection against an already compromised machine, debugger or
malicious extension. Use a trusted IDE and trusted output directory.

## Evidence and failure boundaries

Encryption/decryption are dedicated confidential CLI operations, **outside** the
scientific run/receipt/ledger pipeline. They produce a generic completion status,
not a plaintext input snapshot, source copy, password log, plaintext hash sidecar
or run receipt. Their absence is intentional. They do not alter existing run
records or add a new meaning to `GO_PARANOID` / `COMPLETELY_MAD`.

Callable encryption/decryption functions are available in explicit `GO_PRIVATE`
programs. See [PRIVATE_FUNCTIONS.md](PRIVATE_FUNCTIONS.md) for `input_secret`,
`encrypt_text`, `encrypt_file`, `decrypt_text`, `decrypt_file`, `read_text` and
`display_file`. Ordinary scientific runs still preserve plaintext evidence;
confidential functions refuse in those runs.
You can encrypt a previously completed evidence file; that does not encrypt
or erase its original, backups, swap, snapshots or any already shared copies.

New output files are never overwritten, including existing symlinks and the
input itself. Inputs must be regular files; direct symlinks/directories refuse.
Unix input opens use `O_NOFOLLOW`. The immediate output parent must be an
existing nonsymlink directory; **ancestor directories must be trusted**. This
is not an adversarial-filesystem sandbox or an atomic input snapshot: prevent
concurrent writers if you require one coherent version of a changing input.

Encryption streams through a private temporary ciphertext file. File decryption
writes authenticated chunks to a private, mode-0600 temporary plaintext file in
the destination directory, then publishes it without clobbering only after
complete authentication and sync. Wrong passwords, corruption and truncation
publish no output. Failed temporary files are unlinked on ordinary errors.
A crash/forced termination can leave private temporary files; unlinking is not
secure erasure and disk recovery may recover plaintext. Use encrypted storage
when that risk matters. Windows permissions inherit OS/tempfile policy; Unix
0600 behavior is tested locally. No Windows hardening certification is claimed.

Bounds: 1 GiB plaintext; ciphertext input up to 1 GiB + 2 MiB; 64 KiB age headers;
1 MiB screen previews. scrypt encryption uses fixed `logN=18`, `r=8`, `p=1`
(approximately 256 MiB KDF memory). Decryption permits at most `logN=20`
(approximately 1 GiB). Higher costs refuse; there is no silent downgrade.
The age library supplies OS-backed cryptographic randomness. Equal plaintexts
and passwords intentionally produce different ciphertexts. Visible filenames,
file sizes and the age/scrypt header are not hidden. Metadata is not proof of
sender identity: any holder of the passphrase can create a valid replacement.

## Base64 in Goblin++ programs

```goblin
GO_PARANOID
encoded = str_b64("Hello, π")
restored = str_from_b64(encoded)
print("encoded = {encoded}")
print("restored = {restored}")
seal encoded
```

`str_b64(text)` and alias `str_to_b64(text)` encode UTF-8 using RFC 4648's standard
alphabet and canonical `=` padding. `str_from_b64(text)` decodes canonical
Base64 to UTF-8; it refuses whitespace, URL alphabet, missing/invalid padding,
nonzero padding bits and non-UTF-8 results. Both engines apply the existing
1 MiB text bounds, including expansion. Stored strings do not get interpolated
again. Base64 is **encoding, not encryption**: anyone can decode it. These
ordinary `.gbl` operations retain ordinary plaintext evidence, including their
source and printed results. Use only public demonstration text in the example.

## References

- [age format specification](https://c2sp.org/age@v1.1.0)
- [Rust age implementation](https://docs.rs/age/0.11.5/age/)
- [Reference age implementation](https://github.com/FiloSottile/age)
- [Base64 standard: RFC 4648](https://www.rfc-editor.org/rfc/rfc4648)

Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).

## Dependency review for this local candidate

The 2026-10-31 RustSec check found no reported vulnerabilities after repairing
ZIP 8.6.0 → 9.0.0 and the TLS certificate dependencies (rustls-webpki 0.103.16,
webpki-roots/webpki-root-certs 1.0.10). The maintained classic age 0.11.5 branch
avoids the unused P-256/crypto-bigint dependency from the initial age 0.12.1
candidate. The cipher/file format and scrypt work factor are unchanged.
Two upstream **unmaintained build-dependency** warnings remain: basic-toml and
proc-macro-error2, used through age's build-time localisation macros. They are
recorded, not suppressed; the report is not a proof of vulnerability-free code.
Rust remains 1.92.0. The original audit and repaired audit are retained locally.

The scientific-input policy now records ZIP 9.0.0 as version v2. Exact retained
alpha.41 reader-policy records remain replay-verifiable without rewriting their
receipts. An alpha.41 frozen program still requires an explicit revision and
new freeze before execution under the new policy. Unknown or modified old
policies refuse; verification compatibility is not permission to execute an old
freeze silently.
