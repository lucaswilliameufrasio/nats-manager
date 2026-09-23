use futures_util::TryStreamExt;

#[derive(Clone, Debug)]
pub struct StoredMessage {
    pub sequence: u64,
    pub subject: String,
    pub payload: Vec<u8>,
}

pub async fn list_streams(client: async_nats::Client) -> Result<Vec<String>, String> {
    let context = async_nats::jetstream::new(client);
    let mut names = context.stream_names();
    let mut streams = Vec::new();

    while let Some(name) = names.try_next().await.map_err(|error| error.to_string())? {
        streams.push(name);
    }

    Ok(streams)
}

pub async fn list_consumers(
    client: async_nats::Client,
    stream_name: String,
) -> Result<Vec<String>, String> {
    let context = async_nats::jetstream::new(client);
    let stream = context
        .get_stream(stream_name)
        .await
        .map_err(|error| error.to_string())?;
    let mut names = stream.consumer_names();
    let mut consumers = Vec::new();

    while let Some(name) = names.try_next().await.map_err(|error| error.to_string())? {
        consumers.push(name);
    }

    Ok(consumers)
}

pub async fn create_stream(
    client: async_nats::Client,
    name: String,
    subject: String,
) -> Result<(), String> {
    let context = async_nats::jetstream::new(client);
    context
        .create_stream(async_nats::jetstream::stream::Config {
            name,
            subjects: vec![subject],
            ..Default::default()
        })
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn delete_stream(client: async_nats::Client, name: String) -> Result<(), String> {
    async_nats::jetstream::new(client)
        .delete_stream(name)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn create_pull_consumer(
    client: async_nats::Client,
    stream_name: String,
    consumer_name: String,
) -> Result<(), String> {
    let context = async_nats::jetstream::new(client);
    let stream = context
        .get_stream(stream_name)
        .await
        .map_err(|error| error.to_string())?;
    stream
        .create_consumer(async_nats::jetstream::consumer::pull::Config {
            durable_name: Some(consumer_name.clone()),
            ..Default::default()
        })
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn delete_consumer(
    client: async_nats::Client,
    stream_name: String,
    consumer_name: String,
) -> Result<(), String> {
    let context = async_nats::jetstream::new(client);
    let stream = context
        .get_stream(stream_name)
        .await
        .map_err(|error| error.to_string())?;
    stream
        .delete_consumer(&consumer_name)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn inspect_recent_messages(
    client: async_nats::Client,
    stream_name: String,
    limit: usize,
) -> Result<Vec<StoredMessage>, String> {
    let context = async_nats::jetstream::new(client);
    let stream = context
        .get_stream(stream_name)
        .await
        .map_err(|error| error.to_string())?;
    let info = stream.get_info().await.map_err(|error| error.to_string())?;
    let first = info
        .state
        .last_sequence
        .saturating_sub(limit.saturating_sub(1) as u64)
        .max(info.state.first_sequence);
    let mut messages = Vec::new();

    if info.state.last_sequence == 0 || first > info.state.last_sequence {
        return Ok(messages);
    }

    for sequence in first..=info.state.last_sequence {
        if let Ok(message) = stream.get_raw_message(sequence).await {
            messages.push(StoredMessage {
                sequence: message.sequence,
                subject: message.subject.to_string(),
                payload: message.payload.to_vec(),
            });
        }
    }

    Ok(messages)
}

pub async fn publish(
    client: async_nats::Client,
    subject: String,
    payload: Vec<u8>,
    jetstream: bool,
) -> Result<String, String> {
    if jetstream {
        let ack = async_nats::jetstream::new(client)
            .publish(subject, payload.into())
            .await
            .map_err(|error| error.to_string())?
            .await
            .map_err(|error| error.to_string())?;
        Ok(format!("Published and stored as sequence {}", ack.sequence))
    } else {
        client
            .publish(subject, payload.into())
            .await
            .map_err(|error| error.to_string())?;
        Ok("Message published".to_owned())
    }
}

pub async fn purge_stream(
    client: async_nats::Client,
    stream_name: String,
) -> Result<String, String> {
    let stream = async_nats::jetstream::new(client)
        .get_stream(stream_name)
        .await
        .map_err(|error| error.to_string())?;
    let response = stream.purge().await.map_err(|error| error.to_string())?;
    Ok(format!("Purged {} message(s)", response.purged))
}

pub async fn replay_message(
    client: async_nats::Client,
    subject: String,
    payload: Vec<u8>,
) -> Result<String, String> {
    let ack = async_nats::jetstream::new(client)
        .publish(subject, payload.into())
        .await
        .map_err(|error| error.to_string())?
        .await
        .map_err(|error| error.to_string())?;
    Ok(format!("Message replayed as sequence {}", ack.sequence))
}
