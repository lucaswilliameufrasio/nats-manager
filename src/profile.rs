use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ConnectionProfile {
    pub id: Uuid,
    pub name: String,
    pub servers: Vec<String>,
    pub authentication: Authentication,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TlsConfig {
    pub ca_certificate_key: Option<String>,
    pub client_certificate_key: Option<String>,
    pub client_private_key_key: Option<String>,
    #[serde(default)]
    pub tls_first: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Authentication {
    None,
    UserPassword {
        username: String,
        credential_key: String,
    },
    Token {
        credential_key: String,
    },
    NKey {
        credential_key: String,
    },
    Jwt {
        jwt_credential_key: String,
        seed_credential_key: String,
    },
    CredentialsFile {
        credential_key: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct ProfilesDocument {
    profiles: Vec<ConnectionProfile>,
}

impl ConnectionProfile {
    pub fn new(name: String, servers: Vec<String>, authentication: Authentication) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            servers,
            authentication,
            tls: None,
        }
    }
}

pub struct ProfileStore {
    path: PathBuf,
}

impl ProfileStore {
    pub fn in_directory(directory: impl Into<PathBuf>) -> Self {
        Self {
            path: directory.into().join("profiles.toml"),
        }
    }

    pub fn default_location() -> anyhow::Result<Self> {
        let directories = directories::ProjectDirs::from("io", "nats-manager", "NATS Manager")
            .ok_or_else(|| anyhow::anyhow!("could not locate the application config directory"))?;
        Ok(Self::in_directory(directories.config_dir()))
    }

    pub fn load(&self) -> anyhow::Result<Vec<ConnectionProfile>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let contents = std::fs::read_to_string(&self.path)?;
        let document: ProfilesDocument = toml::from_str(&contents)?;
        Ok(document.profiles)
    }

    pub fn save(&self, profiles: &[ConnectionProfile]) -> anyhow::Result<()> {
        let directory = self
            .path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("profile path has no parent directory"))?;
        std::fs::create_dir_all(directory)?;

        let contents = toml::to_string_pretty(&ProfilesDocument {
            profiles: profiles.to_vec(),
        })?;
        let temporary_path = self.path.with_extension("toml.tmp");
        std::fs::write(&temporary_path, contents)?;
        std::fs::rename(temporary_path, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Authentication, ConnectionProfile, ProfileStore};

    #[test]
    fn profile_serialization_keeps_only_the_keychain_reference() {
        let profile = ConnectionProfile::new(
            "production".to_owned(),
            vec!["tls://nats.example.test:4222".to_owned()],
            Authentication::UserPassword {
                username: "operator".to_owned(),
                credential_key: "profile-secret-ref".to_owned(),
            },
        );

        let serialized = toml::to_string(&profile).expect("profile should serialize");
        assert!(serialized.contains("profile-secret-ref"));
        assert!(!serialized.contains("super-secret-value"));

        let deserialized: ConnectionProfile =
            toml::from_str(&serialized).expect("profile should deserialize");
        assert_eq!(deserialized, profile);
    }

    #[test]
    fn profile_store_round_trip_persists_only_secret_references() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let store = ProfileStore::in_directory(directory.path());
        let profile = ConnectionProfile::new(
            "staging".to_owned(),
            vec!["nats://localhost:4222".to_owned()],
            Authentication::UserPassword {
                username: "service-user".to_owned(),
                credential_key: "keychain-entry-123".to_owned(),
            },
        );

        store
            .save(std::slice::from_ref(&profile))
            .expect("profile should save");
        let persisted = std::fs::read_to_string(directory.path().join("profiles.toml"))
            .expect("profile file should exist");
        assert!(persisted.contains("keychain-entry-123"));
        assert!(!persisted.contains("super-secret-value"));
        assert_eq!(store.load().expect("profiles should load"), vec![profile]);
    }
}
