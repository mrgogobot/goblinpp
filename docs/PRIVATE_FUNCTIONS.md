# Callable file protection - alpha.43

You can call encryption and decryption directly from `.gbl` code and run that
code with the IDE's usual Run/Play action. The same standard age implementation
and failure protections used by the CLI implement these functions.

## First example

Use public practice data and a fresh folder. Save this as `protect.gbl`:

```goblin
GO_PRIVATE

key = input_secret("Passphrase: ")
message = input_secret("Public practice message: ")

encrypt_text(message, key, "secret.age")
decrypt_file("secret.age", key, "restored.txt")
print(read_text("restored.txt"))
```

```sh
goblin++ run protect.gbl
# Native compilation and execution are supported too:
goblin++ run protect.gbl --compile
```

These are alternative runs: use fresh output names/folders when repeating them.
Paths resolve relative to the main `.gbl` source folder under the launcher, even
when you started it from another working directory. Absolute paths also work.
Standalone compiled executables resolve paths from their current working
folder unless the native base directory is set by their launcher.

The example deliberately displays plaintext. For confidential processing,
remove that `print` statement. Never use a public example password for real data.

## Why GO_PRIVATE is explicit

Ordinary Goblin++ science runs preserve source, input responses, arguments,
outputs, evidence and sometimes final variable values. File-protection scripts
must not quietly copy passwords/decrypted content into those records.

`GO_PRIVATE` must be the first main-source statement, before imports. It selects
confidential execution with **no scientific run directory, input replay file,
stdout/stderr log, receipt, ledger update, seal or automatic final-state capture**.
The banner says `GO_PRIVATE` and `CONFIDENTIAL_OPERATION=NO_RUN_RECEIPT`.
`RUN_STATUS=PASS` means the script finished, not that an evidence audit occurred.

It cannot be combined with `GO_PARANOID`, `COMPLETELY_MAD`, numerical audits,
inline Rust, scientific input/export functions or scientific output writers.
Those features need their ordinary evidence machinery. Pure calculations,
strings/Base64, arrays, loops, branches, `g_func`, imports and the confidential
file functions work in private scripts. A science workflow can be completed and
verified normally, then a separate private script can encrypt its files.

A private script refuses if its main source already has a scientific freeze;
it cannot bypass that freeze. `freeze` refuses new private scripts. A static
`check` validates structure without reading secrets, reading/writing data files
or running the script. It does not certify runtime values, passphrases or paths.
The audited library `runtime::run_file` refuses private sources before copying
them; use the command-line private launcher to execute them.

## Function reference

| Function | Meaning and result |
| --- | --- |
| `input_secret()` | Hidden input with a default prompt; returns ordinary in-memory text. |
| `input_secret(prompt)` | Hidden input with a public, control-free prompt (at most 4096 UTF-8 bytes). |
| `encrypt_text(text, key, new_file)` | Encrypt UTF-8 text into a new age file. Returns `true` on success. |
| `encrypt_file(input_file, key, new_file)` | Encrypt exact bytes from a regular file. Returns `true`. |
| `decrypt_file(input_file, key, new_file)` | Authenticate completely before publishing exact plaintext bytes to a new file. Returns `true`. |
| `decrypt_text(input_file, key)` | Authenticate bounded plaintext in memory, then return a UTF-8 string. At most 1 MiB. No plaintext file is created. |
| `read_text(input_file)` | Read at most 1 MiB UTF-8 from a regular file; return raw text. |
| `display_file(input_file)` | Explicit bounded UTF-8 screen preview with terminal controls escaped. Returns `true`. |

Arguments other than prompts are required exactly as shown and must be text.
Errors stop the script; failure does not return a misleading success value.
Encryption accepts passwords of 12-4096 UTF-8 bytes. Newlines, carriage returns
and NUL are refused. Use long, unique, randomly generated passphrases. Decryption
supports shorter historical passwords. Output paths must be new; existing
files, directories, direct symlinks and input/output aliases are refused.
The immediate output parent must exist and not be a symlink. Ancestors must be
trusted. Original input files are never removed.

The age limits remain 1 GiB plaintext, ciphertext input at most 1 GiB + 2 MiB,
64 KiB headers, fixed encryption scrypt logN=18 (about 256 MiB memory), and
maximum decryption logN=20 (about 1 GiB). See `ENCRYPTION.md` for full limits.

`input_secret` accepts 1-4096 UTF-8 response bytes without newline, carriage
return or NUL. It is bounded to 1024 calls, independently of ordinary input
replay events. In a terminal, it uses rpassword's hidden input. In updated IDEs,
it requests a masked dialog and receives the answer directly through stdin.
Do not place secret data in the prompt itself. This is password entry, not a
large-document editor: use `read_text` or `encrypt_file` for larger content.

## File paths from program arguments

Ordinary `argc`/`argv` access remains available in private code. For example:

```goblin
GO_PRIVATE
source_file = argv(1)
key = input_secret("Passphrase: ")
encrypt_file(source_file, key, "from-argument.age")
```

Run it as `goblin++ run protect_file.gbl -- "notes.txt"`. Supply a real path;
a relative path is resolved from the main source folder. Use `input_secret`
for passwords: the IDE/shell may display or retain program arguments.

## Confirm a new password in code

The function does not impose confirmation on every use. Add it when creating
new encrypted files, for example:

```goblin
GO_PRIVATE
key = input_secret("Passphrase: ")
confirmation = input_secret("Confirm: ")
while key != confirmation {
    print("Passphrases differ; enter them again.")
    key = input_secret("Passphrase: ")
    confirmation = input_secret("Confirm: ")
}
encrypt_text("Public practice text", key, "confirmed.age")
```

The CLI/IDE file-action workflows retain their existing automatic encryption
confirmation. This code example makes that choice explicit in a program.

## Reading and displaying decrypted data

```goblin
GO_PRIVATE
key = input_secret("Passphrase: ")
text = decrypt_text("secret.age", key)
print(text)  # Explicitly discloses plaintext to the terminal/IDE console.
```

The returned text is not interpolated a second time. Literal braces from an
input file remain braces. For literal braces in `.gbl` source strings, use the
language's usual `{{` and `}}` escapes. `read_text` and `decrypt_text` return raw
text: `print(text)` does not sanitize terminal controls. To preview a decrypted
file with controls escaped, use `decrypt_file`, then `display_file`.
Binary files must use `encrypt_file` and `decrypt_file`, not `decrypt_text`.

## Use the normal IDE Run button

Install VS Code **0.1.34** or JetBrains **0.3.24**, and install/select the matching
**alpha.43 executable**. Completion, hover help and syntax coloring include the
new functions and `GO_PRIVATE`.

- VS Code: open the `.gbl` file in a trusted workspace and press Run/Play (or
  Goblin++: Run Current File). `input_secret` opens a **masked** dialog; its
  response is not added to the Goblin++ Output channel or program arguments.
- CLion/PyCharm/RustRover: open a trusted project, open the `.gbl` file, and use
  the gutter Run action, Run Configuration, or Tools > Goblin++ > Run Current
  File. Secret prompts use a **masked** IDE dialog and a direct process pipe,
  rather than typing a password into the echoing Run console.

Cancelling a secret dialog closes that input; the script refuses when it cannot
read a complete secret. Previously completed operations are not rolled back.
The original four menu-based file actions are still available.

## Privacy limits that code cannot remove

`GO_PRIVATE` is an execution/evidence policy, not encryption or an OS sandbox.
Password and plaintext variables are ordinary strings in memory. They may be
copied by calculations, the runtime, compiler, IDE, debugger, swap or snapshots;
secure erasure is not promised. Anyone who can change the trusted program,
executable, compiler or IDE can make it disclose data.

Do not hardcode real passwords or secret messages in source, put them in argv,
print them, or commit them. Source files and explicit compiled artifacts remain
on disk and can contain any literals you wrote. IDEs may log program arguments
and explicitly printed text. Compilation emits source/support files; private
`run --compile` keeps them in a private temporary build directory with a private
target cache and removes it on ordinary exit. Standalone `compile` intentionally
retains the requested binary and generated support files. A crash may leave
temporary files. Use trusted encrypted storage where residual copies matter.

File decryption uses a private temporary plaintext file until authenticated EOF;
ordinary failures unlink it, but unlinking is not secure erasure. `decrypt_text`
uses bounded memory instead. No operation removes old plaintext/backups.
A script is not a multi-file transaction: earlier successful encryptions or
explicit plaintext writes remain if a later statement fails. Errors are redacted
so values and paths are not echoed into diagnostic output.

The existing CLI argument forms, `str_b64`, `str_to_b64` and `str_from_b64` remain
supported. Base64 still is encoding, not encryption. Private mode does not
upgrade mathematical precision or certify scientific results.

Optional Goblin++ merchandise: https://h4k3rl1f3.myspreadshop.co.uk

## Source control and earlier receipts

Private files are still ordinary files on disk. GO_PRIVATE does not change Git
tracking or prevent manual sharing. For real secrets, write plaintext outside a
repository or into a folder you have explicitly ignored before using it. Review
GitHub Desktop changes before every commit; do not commit passwords, decrypted
outputs or private source literals.

During verification, alpha.39-42 reserved-word rules are selected using the
recorded runtime version together with their parser policy. An old variable or
user function named GO_PRIVATE/input_secret/encrypt_text is not reinterpreted
as a new directive or builtin. Historical parsing is verification-only; review
and explicitly rename conflicting identifiers before new execution.
