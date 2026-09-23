use async_nats::{Client, ConnectOptions};
use std::io::Cursor;
use std::sync::Arc;

use crate::{
    credentials,
    profile::{Authentication, ConnectionProfile},
};

#[derive(Clone, Debug)]
pub enum JetStreamStatus {
    Enabled {
        streams: usize,
        consumers: usize,
        memory_bytes: u64,
        storage_bytes: u64,
        domain: Option<String>,
    },
    Unavailable(String),
}

pub async fn connect_profile(
    profile: ConnectionProfile,
) -> Result<(Client, JetStreamStatus, String), String> {
    let target = profile.servers.join(", ");
    let options = match &profile.authentication {
        Authentication::None => ConnectOptions::new(),
        Authentication::UserPassword {
            username,
            credential_key,
        } => {
            let password = credentials::load(credential_key).map_err(|error| {
                format!("could not read password from system keychain: {error}")
            })?;
            ConnectOptions::with_user_and_password(username.clone(), password)
        }
        Authentication::Token { credential_key } => {
            let token = credentials::load(credential_key)
                .map_err(|error| format!("could not read token from system keychain: {error}"))?;
            ConnectOptions::with_token(token)
        }
        Authentication::NKey { credential_key } => {
            let seed = credentials::load(credential_key).map_err(|error| {
                format!("could not read NKey seed from system keychain: {error}")
            })?;
            ConnectOptions::with_nkey(seed)
        }
        Authentication::Jwt {
            jwt_credential_key,
            seed_credential_key,
        } => {
            let jwt = credentials::load(jwt_credential_key).map_err(|error| {
                format!("could not read user JWT from system keychain: {error}")
            })?;
            let seed = credentials::load(seed_credential_key).map_err(|error| {
                format!("could not read user seed from system keychain: {error}")
            })?;
            let key_pair = Arc::new(
                nkeys::KeyPair::from_seed(&seed)
                    .map_err(|error| format!("invalid NKey seed: {error}"))?,
            );
            ConnectOptions::with_jwt(jwt, move |nonce| {
                let key_pair = Arc::clone(&key_pair);
                async move { key_pair.sign(&nonce).map_err(async_nats::AuthError::new) }
            })
        }
        Authentication::CredentialsFile { credential_key } => {
            let contents = credentials::load(credential_key).map_err(|error| {
                format!("could not read credentials from system keychain: {error}")
            })?;
            ConnectOptions::with_credentials(&contents)
                .map_err(|error| format!("invalid NATS credentials: {error}"))?
        }
    };
    let options = if let Some(tls) = &profile.tls {
        let options = options
            .tls_client_config(build_tls_config(tls)?)
            .require_tls(true);
        if tls.tls_first {
            options.tls_first()
        } else {
            options
        }
    } else {
        options
    };

    let client = options
        .connect(profile.servers.as_slice())
        .await
        .map_err(|error| error.to_string())?;

    let jetstream_status = match async_nats::jetstream::new(client.clone())
        .query_account()
        .await
    {
        Ok(account) => JetStreamStatus::Enabled {
            streams: account.streams,
            consumers: account.consumers,
            memory_bytes: account.memory,
            storage_bytes: account.storage,
            domain: account.domain,
        },
        Err(error) => JetStreamStatus::Unavailable(error.to_string()),
    };

    Ok((client, jetstream_status, target))
}

fn build_tls_config(tls: &crate::profile::TlsConfig) -> Result<rustls::ClientConfig, String> {
    let mut roots =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if let Some(key) = &tls.ca_certificate_key {
        let pem = credentials::load(key).map_err(|error| {
            format!("could not read CA certificate from system keychain: {error}")
        })?;
        let certificates = rustls_pemfile::certs(&mut Cursor::new(pem.as_bytes()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("invalid CA certificate PEM: {error}"))?;
        if certificates.is_empty() {
            return Err("CA certificate PEM contains no certificates".to_owned());
        }
        for certificate in certificates {
            roots
                .add(certificate)
                .map_err(|error| format!("invalid CA certificate: {error}"))?;
        }
    }

    let client_auth = match (&tls.client_certificate_key, &tls.client_private_key_key) {
        (Some(certificate_key), Some(private_key_key)) => {
            let certificate_pem = credentials::load(certificate_key).map_err(|error| {
                format!("could not read client certificate from system keychain: {error}")
            })?;
            let private_key_pem = credentials::load(private_key_key).map_err(|error| {
                format!("could not read client private key from system keychain: {error}")
            })?;
            let certificates = rustls_pemfile::certs(&mut Cursor::new(certificate_pem.as_bytes()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("invalid client certificate PEM: {error}"))?;
            if certificates.is_empty() {
                return Err("client certificate PEM contains no certificates".to_owned());
            }
            let private_key =
                rustls_pemfile::private_key(&mut Cursor::new(private_key_pem.as_bytes()))
                    .map_err(|error| format!("invalid client private key PEM: {error}"))?
                    .ok_or_else(|| "client private key PEM contains no private key".to_owned())?;
            rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_client_auth_cert(certificates, private_key)
                .map_err(|error| format!("invalid mTLS client identity: {error}"))?
        }
        (None, None) => rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
        _ => {
            return Err("both client certificate and private key are required for mTLS".to_owned());
        }
    };

    Ok(client_auth)
}

#[cfg(test)]
mod tests {
    use crate::profile::TlsConfig;

    use super::build_tls_config;

    #[test]
    fn mutual_tls_requires_both_client_certificate_and_private_key() {
        let tls = TlsConfig {
            ca_certificate_key: None,
            client_certificate_key: Some("cert-entry".to_owned()),
            client_private_key_key: None,
            tls_first: false,
        };

        let error = build_tls_config(&tls).expect_err("partial mTLS identity must be rejected");
        assert!(error.contains("both client certificate and private key are required"));
    }
}
