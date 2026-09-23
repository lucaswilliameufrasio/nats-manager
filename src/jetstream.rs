use futures_util::TryStreamExt;

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
