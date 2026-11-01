//! Recipient-only message encryption using the standard age v1 X25519 format.
//! This authenticates ciphertext integrity, not sender identity or message freshness.
use crate::error::{GoblinError, Result};
use age::secrecy::ExposeSecret;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

pub const MAX_TEXT: usize = 524_288;
pub const MAX_ENVELOPE: usize = 1_048_576;

fn error() -> GoblinError {
    GoblinError::new(
        "G903",
        "Recipient encryption failed: check key, authenticated message integrity and size limits. Values and paths are omitted.",
    )
}
pub fn is_function(name: &str) -> bool {
    matches!(
        name,
        "e2e_keygen" | "e2e_public_key" | "e2e_encrypt" | "e2e_decrypt"
    )
}
pub fn arity(name: &str, count: usize) -> Result<()> {
    if matches!(
        (name, count),
        ("e2e_keygen", 1 | 2) | ("e2e_public_key", 1) | ("e2e_encrypt" | "e2e_decrypt", 2)
    ) {
        Ok(())
    } else {
        Err(error())
    }
}
/// Publish only to a new regular file, with private permissions and atomic no-clobber.
pub(crate) fn save_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if !fs::symlink_metadata(parent).map_err(|_| error())?.is_dir() {
        return Err(error());
    }
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| error())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| error())?;
    }
    file.write_all(bytes).map_err(|_| error())?;
    file.as_file().sync_all().map_err(|_| error())?;
    file.persist_noclobber(path).map_err(|_| error())?;
    Ok(())
}
fn identity(path: &Path) -> Result<age::x25519::Identity> {
    let file = crate::crypto_cli::open_regular(path, 1024)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if file.metadata().map_err(|_| error())?.permissions().mode() & 0o077 != 0 {
            return Err(error());
        }
    }
    let mut text = Zeroizing::new(String::new());
    file.take(1025)
        .read_to_string(&mut text)
        .map_err(|_| error())?;
    if text.len() > 1024 {
        return Err(error());
    }
    text.trim().parse().map_err(|_| error())
}
pub fn keygen(path: &Path) -> Result<String> {
    let key = age::x25519::Identity::generate();
    save_new(path, key.to_string().expose_secret().as_bytes())?;
    Ok(key.to_public().to_string())
}
pub fn public_key(path: &Path) -> Result<String> {
    Ok(identity(path)?.to_public().to_string())
}
pub fn encrypt(text: &str, recipient: &str) -> Result<String> {
    if text.len() > MAX_TEXT || recipient.len() > 128 {
        return Err(error());
    }
    let recipient: age::x25519::Recipient = recipient.parse().map_err(|_| error())?;
    let mut bytes = Vec::new();
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|_| error())?;
    let mut writer = encryptor.wrap_output(&mut bytes).map_err(|_| error())?;
    writer.write_all(text.as_bytes()).map_err(|_| error())?;
    writer.finish().map_err(|_| error())?;
    Ok(STANDARD.encode(bytes))
}
pub fn decrypt(envelope: &str, private_key: &Path) -> Result<String> {
    if envelope.len() > MAX_ENVELOPE {
        return Err(error());
    }
    let bytes = STANDARD.decode(envelope).map_err(|_| error())?;
    // A resource preflight only: age still owns all parsing and authentication.
    // Do not let thousands of remote recipient stanzas consume unbounded work.
    if !bytes.windows(5).take(65_536 - 48).any(|w| w == b"\n--- ") {
        return Err(error());
    }
    let key = identity(private_key)?;
    let decryptor = age::Decryptor::new(bytes.as_slice()).map_err(|_| error())?;
    // Never run a remote sender's password KDF, plugins or external key commands.
    if decryptor.is_scrypt() {
        return Err(error());
    }
    let mut reader = decryptor
        .decrypt(std::iter::once(&key as &dyn age::Identity))
        .map_err(|_| error())?;
    let mut plain = Zeroizing::new(Vec::new());
    reader
        .by_ref()
        .take(MAX_TEXT as u64 + 1)
        .read_to_end(&mut plain)
        .map_err(|_| error())?;
    if plain.len() > MAX_TEXT {
        return Err(error());
    }
    // Read to EOF above authenticates the final age chunk before releasing any text.
    String::from_utf8(plain.to_vec()).map_err(|_| error())
}
