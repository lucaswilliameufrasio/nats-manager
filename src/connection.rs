use async_nats::{Client, ConnectOptions};

use crate::{
    credentials,
    profile::{Authentication, ConnectionProfile},
};

#[derive(Clone, Debug)]
pub enum JetStreamStatus {
    Enabled,
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
        Authentication::CredentialsFile { credential_key } => {
            let contents = credentials::load(credential_key).map_err(|error| {
                format!("could not read credentials from system keychain: {error}")
            })?;
            ConnectOptions::with_credentials(&contents)
                .map_err(|error| format!("invalid NATS credentials: {error}"))?
        }
    };

    let client = options
        .connect(profile.servers.as_slice())
        .await
        .map_err(|error| error.to_string())?;

    let jetstream_status = match async_nats::jetstream::new(client.clone())
        .query_account()
        .await
    {
        Ok(_) => JetStreamStatus::Enabled,
        Err(error) => JetStreamStatus::Unavailable(error.to_string()),
    };

    Ok((client, jetstream_status, target))
}
