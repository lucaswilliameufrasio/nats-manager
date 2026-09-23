use std::path::{Path, PathBuf};

use base64::Engine;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    credentials,
    profile::{Authentication, ConnectionProfile, TlsConfig},
};

#[derive(Debug)]
pub struct ImportedContext {
    pub profile: ConnectionProfile,
    pub credential_keys: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct NatsCliContext {
    #[serde(default)]
    description: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    user: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    token: String,
    #[serde(default)]
    creds: String,
    #[serde(default)]
    nkey: String,
    #[serde(default)]
    user_jwt: String,
    #[serde(default)]
    user_seed: String,
    #[serde(default)]
    cert: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    ca: String,
    #[serde(default)]
    tls_first: bool,
}

pub fn import_context(path: impl AsRef<Path>) -> Result<ImportedContext, String> {
    let path = path.as_ref();
    let context: NatsCliContext = serde_json::from_slice(
        &std::fs::read(path).map_err(|error| format!("could not read NATS context: {error}"))?,
    )
    .map_err(|error| format!("invalid NATS CLI context JSON: {error}"))?;
    if context.url.trim().is_empty() {
        return Err("NATS context does not contain a server URL".to_owned());
    }
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut credential_keys = Vec::new();

    let imported = (|| {
        let authentication = if !context.creds.is_empty() {
            let content = load_material(&context.creds, base_dir, MaterialKind::Credentials)?;
            Authentication::CredentialsFile {
                credential_key: save_secret(&content, "creds", &mut credential_keys)?,
            }
        } else if !context.user_jwt.is_empty() && !context.user_seed.is_empty() {
            let jwt = load_material(&context.user_jwt, base_dir, MaterialKind::Jwt)?;
            let seed = load_material(&context.user_seed, base_dir, MaterialKind::Seed)?;
            Authentication::Jwt {
                jwt_credential_key: save_secret(&jwt, "jwt", &mut credential_keys)?,
                seed_credential_key: save_secret(&seed, "seed", &mut credential_keys)?,
            }
        } else if !context.nkey.is_empty() {
            let seed = load_material(&context.nkey, base_dir, MaterialKind::Seed)?;
            Authentication::NKey {
                credential_key: save_secret(&seed, "nkey", &mut credential_keys)?,
            }
        } else if !context.user.is_empty() && !context.password.is_empty() {
            reject_external_resolver(&context.password)?;
            Authentication::UserPassword {
                username: context.user.clone(),
                credential_key: save_secret(&context.password, "password", &mut credential_keys)?,
            }
        } else if !context.token.is_empty() {
            reject_external_resolver(&context.token)?;
            Authentication::Token {
                credential_key: save_secret(&context.token, "token", &mut credential_keys)?,
            }
        } else {
            Authentication::None
        };

        let ca_certificate_key = import_optional_material(
            &context.ca,
            base_dir,
            MaterialKind::File,
            "ca",
            &mut credential_keys,
        )?;
        let client_certificate_key = import_optional_material(
            &context.cert,
            base_dir,
            MaterialKind::File,
            "client-cert",
            &mut credential_keys,
        )?;
        let client_private_key_key = import_optional_material(
            &context.key,
            base_dir,
            MaterialKind::File,
            "client-key",
            &mut credential_keys,
        )?;

        let tls = if ca_certificate_key.is_some()
            || client_certificate_key.is_some()
            || client_private_key_key.is_some()
            || context.tls_first
        {
            Some(TlsConfig {
                ca_certificate_key,
                client_certificate_key,
                client_private_key_key,
                tls_first: context.tls_first,
            })
        } else {
            None
        };
        let name = if context.description.trim().is_empty() {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Imported NATS context")
                .to_owned()
        } else {
            context.description
        };
        let servers = context
            .url
            .split(',')
            .map(str::trim)
            .filter(|server| !server.is_empty())
            .map(str::to_owned)
            .collect();
        let mut profile = ConnectionProfile::new(name, servers, authentication);
        profile.tls = tls;
        Ok(profile)
    })();

    match imported {
        Ok(profile) => Ok(ImportedContext {
            profile,
            credential_keys,
        }),
        Err(error) => {
            for key in credential_keys {
                let _ = credentials::delete(&key);
            }
            Err(error)
        }
    }
}

#[derive(Clone, Copy)]
enum MaterialKind {
    Credentials,
    Jwt,
    Seed,
    File,
}

fn load_material(value: &str, base_dir: &Path, kind: MaterialKind) -> Result<String, String> {
    reject_external_resolver(value)?;
    if let Some(encoded) = value.strip_prefix("data:;base64,") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| format!("invalid embedded NATS context data: {error}"))?;
        return String::from_utf8(bytes)
            .map_err(|error| format!("embedded NATS context data is not UTF-8: {error}"));
    }

    let path_value = value.strip_prefix("file://").unwrap_or(value);
    let path = expand_path(path_value, base_dir)?;
    if path.is_file() {
        return std::fs::read_to_string(path)
            .map_err(|error| format!("could not read credential material: {error}"));
    }

    let embedded_creds = matches!(kind, MaterialKind::Credentials)
        && value.contains("-----BEGIN NATS USER JWT-----");
    let inline_jwt = matches!(kind, MaterialKind::Jwt) && value.starts_with("eyJ");
    let inline_seed = matches!(kind, MaterialKind::Seed) && value.starts_with('S');
    if embedded_creds || inline_jwt || inline_seed {
        return Ok(value.to_owned());
    }

    Err("credential path in NATS context could not be resolved".to_owned())
}

fn expand_path(value: &str, base_dir: &Path) -> Result<PathBuf, String> {
    let expanded = if let Some(rest) = value.strip_prefix("~/") {
        let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_owned())?;
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(value)
    };
    Ok(if expanded.is_absolute() {
        expanded
    } else {
        base_dir.join(expanded)
    })
}

fn reject_external_resolver(value: &str) -> Result<(), String> {
    if ["env://", "op://", "nsc://"]
        .iter()
        .any(|scheme| value.starts_with(scheme))
    {
        return Err(
            "NATS context uses an external credential resolver; resolve it before importing"
                .to_owned(),
        );
    }
    Ok(())
}

fn save_secret(value: &str, label: &str, keys: &mut Vec<String>) -> Result<String, String> {
    let key = format!("{}:{label}", Uuid::new_v4());
    credentials::store(&key, value)
        .map_err(|error| format!("could not store imported secret in system keychain: {error}"))?;
    keys.push(key.clone());
    Ok(key)
}

fn import_optional_material(
    value: &str,
    base_dir: &Path,
    kind: MaterialKind,
    label: &str,
    keys: &mut Vec<String>,
) -> Result<Option<String>, String> {
    if value.is_empty() {
        return Ok(None);
    }
    let content = load_material(value, base_dir, kind)?;
    save_secret(&content, label, keys).map(Some)
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::{MaterialKind, NatsCliContext, load_material, reject_external_resolver};

    #[test]
    fn nats_cli_context_fields_match_the_supported_import_shape() {
        let context: NatsCliContext = serde_json::from_str(
            r#"{"description":"production","url":"tls://nats.example:4222","user":"agent","password":"not-in-profile"}"#,
        )
        .expect("context JSON should deserialize");
        assert_eq!(context.description, "production");
        assert_eq!(context.url, "tls://nats.example:4222");
        assert_eq!(context.user, "agent");
    }

    #[test]
    fn importer_rejects_external_secret_resolvers_without_executing_them() {
        assert!(reject_external_resolver("env://NATS_PASSWORD").is_err());
        assert!(reject_external_resolver("op://vault/item/field").is_err());
        assert!(reject_external_resolver("nsc://operator/account/user").is_err());
        assert!(reject_external_resolver("file:///tmp/user.creds").is_ok());
    }

    #[test]
    fn importer_reads_relative_files_and_embedded_material() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        std::fs::write(directory.path().join("user.creds"), "creds-content")
            .expect("credential fixture should be written");
        let relative = load_material("user.creds", directory.path(), MaterialKind::Credentials)
            .expect("relative credential path should load");
        assert_eq!(relative, "creds-content");

        let encoded = base64::engine::general_purpose::STANDARD.encode("embedded-creds");
        let embedded = load_material(
            &format!("data:;base64,{encoded}"),
            directory.path(),
            MaterialKind::Credentials,
        )
        .expect("embedded credential data should decode");
        assert_eq!(embedded, "embedded-creds");
    }
}
