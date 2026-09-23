use futures_util::TryStreamExt;

#[derive(Clone, Debug)]
pub struct StoredMessage {
    pub sequence: u64,
    pub subject: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct StreamDetails {
    pub subjects: Vec<String>,
    pub messages: u64,
    pub bytes: u64,
    pub consumers: usize,
}

#[derive(Clone, Debug)]
pub struct ConsumerDetails {
    pub ack_wait_seconds: u64,
    pub max_deliver: i64,
}

fn jetstream_context(
    client: async_nats::Client,
    api_prefix: Option<String>,
) -> async_nats::jetstream::Context {
    if let Some(prefix) = api_prefix {
        async_nats::jetstream::ContextBuilder::new()
            .api_prefix(prefix)
            .build(client)
    } else {
        async_nats::jetstream::new(client)
    }
}

pub async fn list_streams(
    client: async_nats::Client,
    api_prefix: Option<String>,
) -> Result<Vec<String>, String> {
    let context = jetstream_context(client, api_prefix);
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
    api_prefix: Option<String>,
) -> Result<Vec<String>, String> {
    let context = jetstream_context(client, api_prefix);
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
    api_prefix: Option<String>,
) -> Result<(), String> {
    let context = jetstream_context(client, api_prefix);
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

pub async fn describe_stream(
    client: async_nats::Client,
    name: String,
    api_prefix: Option<String>,
) -> Result<StreamDetails, String> {
    let stream = jetstream_context(client, api_prefix)
        .get_stream(name)
        .await
        .map_err(|error| error.to_string())?;
    let info = stream.get_info().await.map_err(|error| error.to_string())?;
    Ok(StreamDetails {
        subjects: info.config.subjects,
        messages: info.state.messages,
        bytes: info.state.bytes,
        consumers: info.state.consumer_count,
    })
}

pub async fn update_stream_subject(
    client: async_nats::Client,
    name: String,
    subject: String,
    api_prefix: Option<String>,
) -> Result<(), String> {
    let context = jetstream_context(client, api_prefix);
    let stream = context
        .get_stream(name)
        .await
        .map_err(|error| error.to_string())?;
    let mut info = stream.get_info().await.map_err(|error| error.to_string())?;
    info.config.subjects = vec![subject];
    context
        .update_stream(info.config)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn delete_stream(
    client: async_nats::Client,
    name: String,
    api_prefix: Option<String>,
) -> Result<(), String> {
    jetstream_context(client, api_prefix)
        .delete_stream(name)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn create_pull_consumer(
    client: async_nats::Client,
    stream_name: String,
    consumer_name: String,
    api_prefix: Option<String>,
) -> Result<(), String> {
    let context = jetstream_context(client, api_prefix);
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

pub async fn update_consumer_limits(
    client: async_nats::Client,
    stream_name: String,
    consumer_name: String,
    max_deliver: i64,
    ack_wait_seconds: u64,
    api_prefix: Option<String>,
) -> Result<(), String> {
    if max_deliver < 0 {
        return Err("maximum deliveries cannot be negative".to_owned());
    }
    let stream = jetstream_context(client, api_prefix)
        .get_stream(stream_name)
        .await
        .map_err(|error| error.to_string())?;
    let mut info = stream
        .consumer_info(&consumer_name)
        .await
        .map_err(|error| error.to_string())?;
    info.config.max_deliver = max_deliver;
    info.config.ack_wait = std::time::Duration::from_secs(ack_wait_seconds);
    stream
        .update_consumer(info.config)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub async fn describe_consumer(
    client: async_nats::Client,
    stream_name: String,
    consumer_name: String,
    api_prefix: Option<String>,
) -> Result<ConsumerDetails, String> {
    let stream = jetstream_context(client, api_prefix)
        .get_stream(stream_name)
        .await
        .map_err(|error| error.to_string())?;
    let info = stream
        .consumer_info(&consumer_name)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ConsumerDetails {
        ack_wait_seconds: info.config.ack_wait.as_secs(),
        max_deliver: info.config.max_deliver,
    })
}

pub async fn delete_consumer(
    client: async_nats::Client,
    stream_name: String,
    consumer_name: String,
    api_prefix: Option<String>,
) -> Result<(), String> {
    let context = jetstream_context(client, api_prefix);
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
    api_prefix: Option<String>,
) -> Result<Vec<StoredMessage>, String> {
    let context = jetstream_context(client, api_prefix);
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
    api_prefix: Option<String>,
) -> Result<String, String> {
    if jetstream {
        let ack = jetstream_context(client, api_prefix)
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
    api_prefix: Option<String>,
) -> Result<String, String> {
    let stream = jetstream_context(client, api_prefix)
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
    api_prefix: Option<String>,
) -> Result<String, String> {
    let ack = jetstream_context(client, api_prefix)
        .publish(subject, payload.into())
        .await
        .map_err(|error| error.to_string())?
        .await
        .map_err(|error| error.to_string())?;
    Ok(format!("Message replayed as sequence {}", ack.sequence))
}
