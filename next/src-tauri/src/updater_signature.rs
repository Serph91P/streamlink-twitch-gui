use std::{fs::File, io::Read, path::Path};

use base64::{Engine, engine::general_purpose::STANDARD};
use minisign_verify::{PublicKey, Signature};

pub fn verify_updater_signature(
    public_key: &str,
    artifact_path: &Path,
    signature_path: &Path,
) -> Result<(), String> {
    if public_key.trim().is_empty() {
        return Err("TAURI_UPDATER_PUBLIC_KEY is missing".to_owned());
    }
    let public_key = decode_tauri_value(public_key, "TAURI_UPDATER_PUBLIC_KEY")?;
    let public_key = PublicKey::decode(&public_key)
        .map_err(|error| format!("TAURI_UPDATER_PUBLIC_KEY is invalid: {error}"))?;
    let signature = std::fs::read_to_string(signature_path)
        .map_err(|error| format!("invalid updater signature: {error}"))?;
    let signature = decode_tauri_value(&signature, "updater signature")?;
    let signature = Signature::decode(&signature)
        .map_err(|error| format!("invalid updater signature: {error}"))?;
    let mut verifier = public_key
        .verify_stream(&signature)
        .map_err(|error| format!("invalid updater signature: {error}"))?;
    let mut artifact = File::open(artifact_path)
        .map_err(|error| format!("could not read updater artifact: {error}"))?;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = artifact
            .read(&mut buffer)
            .map_err(|error| format!("could not read updater artifact: {error}"))?;
        if count == 0 {
            break;
        }
        verifier.update(&buffer[..count]);
    }
    verifier
        .finalize()
        .map_err(|error| format!("updater signature verification failed: {error}"))
}

fn decode_tauri_value(value: &str, label: &str) -> Result<String, String> {
    let decoded = STANDARD
        .decode(value.trim())
        .map_err(|_| format!("{label} is not valid base64"))?;
    String::from_utf8(decoded).map_err(|_| format!("{label} is not valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::verify_updater_signature;
    use base64::{Engine, engine::general_purpose::STANDARD};

    const PUBLIC_KEY: &str = "untrusted comment: minisign public key E7620F1842B4E81F\n\
RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3\n";
    const SIGNATURE: &str = "untrusted comment: signature from minisign secret key\n\
RUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\n\
trusted comment: timestamp:1556193335\tfile:test\n\
y/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==\n";

    fn fixture_paths(name: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "streamlink-twitch-gui-updater-signature-{name}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("artifact");
        let signature = root.join("artifact.sig");
        fs::write(&signature, STANDARD.encode(SIGNATURE)).unwrap();
        (artifact, signature)
    }

    #[test]
    fn accepts_signature_for_exact_artifact_bytes() {
        let (artifact, signature) = fixture_paths("valid");
        fs::write(&artifact, b"test").unwrap();

        verify_updater_signature(&STANDARD.encode(PUBLIC_KEY), &artifact, &signature).unwrap();

        fs::remove_dir_all(artifact.parent().unwrap()).unwrap();
    }

    #[test]
    fn rejects_signature_for_different_artifact_bytes() {
        let (artifact, signature) = fixture_paths("invalid");
        fs::write(&artifact, b"fabricated").unwrap();

        let error = verify_updater_signature(&STANDARD.encode(PUBLIC_KEY), &artifact, &signature)
            .unwrap_err();
        assert!(error.contains("verification failed"));

        fs::remove_dir_all(artifact.parent().unwrap()).unwrap();
    }
}
