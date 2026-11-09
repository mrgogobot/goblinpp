//! Confidential file operations are deliberately outside the scientific run/receipt pipeline.
//! The age crate owns the file format, authenticated streaming and cryptographic randomness.
use crate::error::{GoblinError, Result};
use age::secrecy::SecretString;
use clap::Args;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use zeroize::Zeroizing;

pub const MAX_PLAINTEXT: u64 = 1_073_741_824;
pub const MAX_CIPHERTEXT: u64 = MAX_PLAINTEXT + 2_097_152;
pub const MAX_PREVIEW: u64 = 1_048_576;
const KEY_LIMIT: u64 = 4098;
const MAX_HEADER_BYTES: u64 = 65_536;

// Limit untrusted header parsing separately from the streaming payload. The
// allowance is lifted only after age has successfully parsed the bounded header.
struct HeaderLimited<R> {
    reader: R,
    remaining: std::rc::Rc<std::cell::Cell<u64>>,
}
impl<R: Read> Read for HeaderLimited<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let remaining = self.remaining.get();
        if remaining == 0 {
            return Err(io::Error::other("Encrypted header exceeds limit."));
        }
        let size = buffer.len().min(remaining as usize);
        let count = self.reader.read(&mut buffer[..size])?;
        self.remaining.set(remaining - count as u64);
        Ok(count)
    }
}
// N=2^18, r=8, p=1: approximately 256 MiB scrypt memory; no machine-dependent downgrade.
const SCRYPT_LOG_N: u8 = 18;
const MAX_SCRYPT_LOG_N: u8 = 20;

#[derive(Args)]
pub struct CryptoArgs {
    /// Existing regular input file. Symlinks are refused.
    #[arg(long, conflicts_with_all = ["text", "text_stdin"])]
    pub input: Option<PathBuf>,
    /// New output file; existing files and symlinks are never replaced.
    #[arg(long, conflicts_with = "show")]
    pub output: Option<PathBuf>,
    /// Authenticate fully, then display bounded UTF-8 with terminal controls escaped.
    #[arg(long)]
    pub show: bool,
    /// UNSAFE: literal password can leak through shell history/process listings. Prefer the hidden prompt.
    #[arg(long, conflicts_with = "key_stdin")]
    pub key: Option<String>,
    /// Read password as the first UTF-8 stdin line (for IDEs/pipes). Never echo or record it.
    #[arg(long)]
    pub key_stdin: bool,
    /// Read plaintext from stdin, after the password line when --key-stdin is used.
    #[arg(long, conflicts_with = "text")]
    pub text_stdin: bool,
    /// Plaintext to encrypt. UNSAFE for secrets: a shell can retain literal text in history.
    pub text: Option<String>,
}
impl std::fmt::Debug for CryptoArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CryptoArgs { values: REDACTED }")
    }
}
fn error(message: &str) -> GoblinError {
    GoblinError::new("G902", message)
}
fn io_error(_: io::Error) -> GoblinError {
    error(
        "File operation failed; supplied paths and contents are omitted. No completed output was published.",
    )
}

/// Only normalize flags for the dedicated crypto invocation, never a .gbl program's argv.
pub fn normalize_shortcuts(args: &mut [String]) {
    let Some(first) = args.get_mut(1) else { return };
    match first.as_str() {
        "-enc" | "--enc" => *first = "encrypt".into(),
        "-dec" | "--dec" => *first = "decrypt".into(),
        "-of" | "--of" => *first = "read-file".into(),
        _ => {}
    }
    if !args
        .get(1)
        .is_some_and(|s| s == "encrypt" || s == "decrypt")
    {
        return;
    }
    // Skip option values: a password or filename equal to '-nf' must stay literal.
    let mut value_next = false;
    for arg in args.iter_mut().skip(2) {
        if value_next {
            value_next = false;
            continue;
        }
        if arg == "--" {
            break;
        }
        *arg = match arg.as_str() {
            "-key" => "--key".into(),
            "-nf" | "--nf" => "--output".into(),
            "-of" | "--of" => "--input".into(),
            _ => arg.clone(),
        };
        value_next = ["--key", "--input", "--output"].contains(&arg.as_str());
    }
}

pub(crate) fn open_regular(path: &Path, limit: u64) -> Result<File> {
    if !fs::symlink_metadata(path)
        .map_err(io_error)?
        .file_type()
        .is_file()
    {
        return Err(error(
            "Input must be a regular file; directories and symlinks are refused.",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(io_error)?;
    let meta = file.metadata().map_err(io_error)?;
    if !meta.is_file() || meta.len() > limit {
        return Err(error(
            "Input is not a regular file or exceeds the documented size limit.",
        ));
    }
    Ok(file)
}
pub(crate) fn destination(path: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(_) => return Err(error("Output already exists. Refusing to overwrite it.")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_error(e)),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !fs::symlink_metadata(parent).map_err(io_error)?.is_dir() {
        return Err(error(
            "Output parent must be an existing directory, not a symlink.",
        ));
    }
    Ok(parent.to_path_buf())
}
fn copy_bounded(mut input: impl Read, mut output: impl Write, limit: u64) -> Result<u64> {
    let mut buffer = Zeroizing::new([0u8; 65536]);
    let mut total = 0u64;
    loop {
        let count = input.read(&mut buffer[..]).map_err(io_error)?;
        if count == 0 {
            return Ok(total);
        }
        total = total
            .checked_add(count as u64)
            .ok_or_else(|| error("File size overflow."))?;
        if total > limit {
            return Err(error("File exceeds the documented size limit."));
        }
        output.write_all(&buffer[..count]).map_err(io_error)?;
    }
}
// Prompt text uses the already-open stderr stream. The standard library still
// opens/controls the input TTY and owns masking/restoration; no custom echo code.
pub(crate) fn hidden_password(prompt: &str) -> io::Result<String> {
    rpassword::prompt_password_with_config(
        prompt,
        rpassword::ConfigBuilder::new()
            .output_writer(io::stderr())
            .password_feedback_hide()
            .build(),
    )
}
fn read_password(
    args: &mut CryptoArgs,
    stdin: &mut impl BufRead,
    encrypt: bool,
) -> Result<SecretString> {
    let password = if let Some(value) = args.key.take() {
        eprintln!(
            "WARNING: --key exposes the password to shell history/process listings. Use the hidden prompt instead."
        );
        Zeroizing::new(value)
    } else if args.key_stdin {
        let mut value = Zeroizing::new(String::new());
        stdin
            .take(KEY_LIMIT)
            .read_line(&mut value)
            .map_err(io_error)?;
        if !value.ends_with('\n') || value.len() >= KEY_LIMIT as usize {
            return Err(error(
                "Password stdin requires one bounded UTF-8 line ending in a newline.",
            ));
        }
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
        value
    } else {
        let value = Zeroizing::new(hidden_password("Password (hidden): ").map_err(|_| error("A terminal is required for the hidden password prompt. IDEs/pipes may use --key-stdin."))?);
        if encrypt {
            let confirmation =
                Zeroizing::new(hidden_password("Confirm password (hidden): ").map_err(io_error)?);
            if *value != *confirmation {
                return Err(error("Password confirmation did not match."));
            }
        }
        value
    };
    validate_password(&password, encrypt)?;
    Ok(SecretString::from(password.to_string()))
}

fn validate_password(password: &str, encrypt: bool) -> Result<()> {
    if password.is_empty() || password.len() > 4096 || password.contains(['\n', '\r', '\0']) {
        return Err(error(
            "Password must contain 1–4096 UTF-8 bytes with no newline or NUL.",
        ));
    }
    if encrypt && password.len() < 12 {
        return Err(error(
            "Use a long, unique passphrase (at least 12 UTF-8 bytes). Length alone does not guarantee strength.",
        ));
    }
    Ok(())
}

pub fn dispatch(mut args: CryptoArgs, encrypt: bool) -> Result<i32> {
    // Validate shape before reading secrets, creating files or invoking the KDF.
    if encrypt && (args.output.is_none() || args.show) {
        return Err(error(
            "Encryption requires --output NEW_FILE; --show is decryption-only.",
        ));
    }
    if !encrypt
        && (args.input.is_none()
            || args.text.is_some()
            || args.text_stdin
            || (args.output.is_some() == args.show))
    {
        return Err(error(
            "Decryption requires --input FILE and exactly one of --output NEW_FILE or --show.",
        ));
    }
    if encrypt && args.input.is_none() && args.text.is_none() && !args.text_stdin {
        return Err(error(
            "Encryption requires --input FILE, literal text, or --text-stdin.",
        ));
    }
    let parent = args.output.as_deref().map(destination).transpose()?;
    let input_limit = if encrypt {
        MAX_PLAINTEXT
    } else {
        MAX_CIPHERTEXT
    };
    let input = args
        .input
        .as_deref()
        .map(|p| open_regular(p, input_limit))
        .transpose()?;
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let password = read_password(&mut args, &mut stdin, encrypt)?;
    let preview = perform(args, encrypt, input, parent, password, &mut stdin)?;
    if let Some(preview) = preview {
        show_text(preview.as_slice())?;
    } else {
        println!(
            "{}_STATUS=PASS\nFORMAT=age-v1\nCONFIDENTIAL_OPERATION=NO_RUN_RECEIPT",
            if encrypt { "ENCRYPTION" } else { "DECRYPTION" }
        );
    }
    Ok(0)
}

fn perform(
    mut args: CryptoArgs,
    encrypt: bool,
    input: Option<File>,
    parent: Option<PathBuf>,
    password: SecretString,
    mut stdin: impl Read,
) -> Result<Option<Zeroizing<Vec<u8>>>> {
    let mut staging = parent
        .map(|parent| NamedTempFile::new_in(parent).map_err(io_error))
        .transpose()?;
    if encrypt {
        let mut recipient = age::scrypt::Recipient::new(password);
        recipient.set_work_factor(SCRYPT_LOG_N);
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
                .map_err(|_| error("Encryption setup failed."))?;
        let mut writer = encryptor
            .wrap_output(staging.as_mut().unwrap().as_file_mut())
            .map_err(io_error)?;
        if let Some(input) = input {
            copy_bounded(input, &mut writer, MAX_PLAINTEXT)?;
        } else if let Some(text) = args.text.take() {
            eprintln!(
                "WARNING: literal plaintext can remain in shell history. Prefer --text-stdin or a file."
            );
            let text = Zeroizing::new(text);
            copy_bounded(text.as_bytes(), &mut writer, MAX_PLAINTEXT)?;
        } else {
            copy_bounded(&mut stdin, &mut writer, MAX_PLAINTEXT)?;
        }
        writer.finish().map_err(io_error)?;
    } else {
        let allowance = std::rc::Rc::new(std::cell::Cell::new(MAX_HEADER_BYTES));
        let limited = HeaderLimited {
            reader: input.unwrap(),
            remaining: allowance.clone(),
        };
        let decryptor = age::Decryptor::new_buffered(io::BufReader::new(limited))
            .map_err(|_| error("Invalid or unsupported age file; no plaintext released."))?;
        allowance.set(MAX_CIPHERTEXT);
        if !decryptor.is_scrypt() {
            return Err(error(
                "This command requires a passphrase-encrypted age file.",
            ));
        }
        let mut identity = age::scrypt::Identity::new(password);
        identity.set_max_work_factor(MAX_SCRYPT_LOG_N);
        let reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))
            .map_err(|_| error("Decryption refused: wrong password, damaged file, or KDF cost beyond the supported limit. No plaintext released."))?;
        if args.show {
            // Screen previews authenticate completely in bounded, zeroized memory;
            // they never create a plaintext temporary file.
            let mut preview = Zeroizing::new(Vec::new());
            copy_bounded(reader, &mut *preview, MAX_PREVIEW)
                .map_err(|_| error("Decryption/authentication failed or preview exceeds 1 MiB. No plaintext released."))?;
            return Ok(Some(preview));
        }
        // age authenticates chunks. Retain ALL plaintext privately until authenticated EOF,
        // so truncation or a damaged final chunk cannot publish an accepted prefix.
        copy_bounded(reader, staging.as_mut().unwrap().as_file_mut(), MAX_PLAINTEXT)
            .map_err(|_| error("Decryption/authentication failed or plaintext exceeds the limit. No plaintext released."))?;
    }
    staging
        .as_mut()
        .unwrap()
        .as_file_mut()
        .sync_all()
        .map_err(io_error)?;
    if let Some(output) = args.output {
        staging.unwrap().persist_noclobber(output).map_err(|_| error("Output could not be published without overwriting. No completed output was published."))?;
    }
    Ok(None)
}

fn show_text(input: impl Read) -> Result<()> {
    let mut bytes = Zeroizing::new(Vec::new());
    input
        .take(MAX_PREVIEW + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() as u64 > MAX_PREVIEW {
        return Err(error(
            "Screen preview exceeds 1 MiB. Decrypt/read to a file instead.",
        ));
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| error("Screen preview requires UTF-8 text. Use a file for binary data."))?;
    let mut stdout = io::stdout().lock();
    for ch in text.chars() {
        // Preserve only line breaks and horizontal tabs; never emit terminal escape sequences,
        // bidi formatting controls, C1 controls, or other invisible format characters.
        if (ch.is_control() && ch != '\n' && ch != '\t')
            || matches!(ch, '\u{061c}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}')
        {
            write!(stdout, "\\u{{{:x}}}", ch as u32).map_err(io_error)?;
        } else {
            write!(stdout, "{ch}").map_err(io_error)?;
        }
    }
    stdout.flush().map_err(io_error)
}
pub fn read_file(path: &Path) -> Result<i32> {
    show_text(open_regular(path, MAX_PREVIEW)?)?;
    Ok(0)
}

/// Same authenticated file operation as the CLI, without printing or recording secrets.
pub(crate) fn script_operation(
    input: Option<&Path>,
    text: Option<&str>,
    password: &str,
    output: Option<&Path>,
    encrypt: bool,
) -> Result<Option<String>> {
    validate_password(password, encrypt)?;
    let parent = output.map(destination).transpose()?;
    let input = input
        .map(|p| {
            open_regular(
                p,
                if encrypt {
                    MAX_PLAINTEXT
                } else {
                    MAX_CIPHERTEXT
                },
            )
        })
        .transpose()?;
    let args = CryptoArgs {
        input: None,
        output: output.map(Path::to_path_buf),
        show: output.is_none(),
        key: None,
        key_stdin: false,
        text_stdin: false,
        text: None,
    };
    // Borrow text through stdin instead of making an unnecessary plaintext String copy.
    let preview = perform(
        args,
        encrypt,
        input,
        parent,
        SecretString::from(password.to_owned()),
        text.unwrap_or("").as_bytes(),
    )?;
    preview
        .map(|bytes| {
            std::str::from_utf8(&bytes).map(str::to_owned).map_err(|_| {
                error("Decrypted text is not UTF-8; use decrypt_file() for exact binary bytes.")
            })
        })
        .transpose()
}

pub(crate) fn script_read_text(path: &Path) -> Result<String> {
    let mut bytes = Zeroizing::new(Vec::new());
    copy_bounded(open_regular(path, MAX_PREVIEW)?, &mut *bytes, MAX_PREVIEW)?;
    std::str::from_utf8(&bytes)
        .map(str::to_owned)
        .map_err(|_| error("read_text() requires UTF-8; binary files cannot be displayed as text."))
}
