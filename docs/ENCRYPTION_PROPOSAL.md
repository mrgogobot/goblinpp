# Proposal: optional encrypted export

**Status: proposed, not implemented.** Goblin++ currently hashes and preserves
evidence; hashing detects changes but does not hide file contents. `GO_PARANOID`
is not an encryption switch. Until this proposal is built and tested, ordinary
run directories, imported-data copies, input responses, arguments, and logs
must be treated as plaintext.

## First useful scope

Offer explicit CLI operations to encrypt and decrypt either one file or a
single directory snapshot (for example, a complete run bundle). Never silently
encrypt normal run output or change the bytes of an existing freeze or receipt.
The encrypted export should carry a manifest *inside the encrypted payload*
recording the source bundle's hashes, format version, and creation parameters.
Do not publish plaintext hashes, private paths, or secrets as sidecar metadata.
Preserve the original evidence as-is unless the user explicitly decides how
to manage it separately.

Use an established, independently reviewed encryption format and maintained
implementation, such as [age](https://github.com/FiloSottile/age), rather than
designing a Goblin++ cipher. Decide explicitly between recipient-key and
passphrase workflows, including how a lost key is handled. Never place a
passphrase or private key in a `.gbl` program, command-line argument, run
receipt, log, or repository. A future implementation needs a clear statement of
what remains visible (for example, output filename, file size, and any
unencrypted metadata).

## Required acceptance tests before claiming confidentiality

- File and directory round trips recover identical bytes and pass the ordinary
  Goblin++ verification of the restored run bundle.
- Wrong key, wrong passphrase, modified ciphertext, and truncated ciphertext
  fail closed; no unauthenticated plaintext is accepted.
- Existing destinations are never overwritten implicitly; failed operations
  leave no deceptively complete output.
- Directory inputs reject symlinks and unsafe paths; extraction cannot write
  outside the chosen destination. Empty, nested, Unicode-named, and large-file
  cases are tested without unbounded memory use.
- Keys and passphrases never appear in process arguments, program source,
  receipts, logs, crash output, or test fixtures.
- The documentation states that encrypting an **export** does not encrypt the
  original run directory, temporary files, backups, or a user's full disk.

This can follow a limited public alpha. If encrypted data handling is promised
for the *first* public release, these tests and a security review become
release gates instead of roadmap items.
