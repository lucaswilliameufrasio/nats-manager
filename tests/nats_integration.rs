use futures_util::StreamExt;
use nats_manager::{
    connection::{self, JetStreamStatus},
    jetstream,
    profile::{Authentication, ConnectionProfile},
};
use uuid::Uuid;

fn profile_for(url: String) -> ConnectionProfile {
    ConnectionProfile::new(
        "integration-test".to_owned(),
        vec![url],
        Authentication::None,
    )
}

#[tokio::test]
async fn jetstream_stream_consumer_message_and_purge_flow() {
    let Ok(url) = std::env::var("NATS_URL") else {
        eprintln!("Skipping JetStream integration test: NATS_URL is not set");
        return;
    };
    let (client, status, _) = connection::connect_profile(profile_for(url))
        .await
        .expect("should connect to NATS");
    assert!(matches!(status, JetStreamStatus::Enabled { .. }));

    let unique = Uuid::new_v4().simple().to_string();
    let stream = format!("NATS_MANAGER_TEST_{unique}");
    let subject = format!("nats_manager_test.{unique}");
    let consumer = format!("nats_manager_consumer_{unique}");

    jetstream::create_stream(client.clone(), stream.clone(), subject.clone())
        .await
        .expect("stream should be created");
    assert!(
        jetstream::list_streams(client.clone())
            .await
            .expect("stream listing should work")
            .contains(&stream)
    );

    jetstream::create_pull_consumer(client.clone(), stream.clone(), consumer.clone())
        .await
        .expect("consumer should be created");
    assert!(
        jetstream::list_consumers(client.clone(), stream.clone())
            .await
            .expect("consumer listing should work")
            .contains(&consumer)
    );

    jetstream::publish(
        client.clone(),
        subject.clone(),
        b"first payload".to_vec(),
        true,
    )
    .await
    .expect("message should be published and acknowledged");
    let messages = jetstream::inspect_recent_messages(client.clone(), stream.clone(), 20)
        .await
        .expect("messages should be inspectable");
    let message = messages
        .first()
        .expect("published message should be present");
    assert_eq!(message.subject, subject);
    assert_eq!(message.payload, b"first payload");

    jetstream::replay_message(
        client.clone(),
        message.subject.clone(),
        message.payload.clone(),
    )
    .await
    .expect("message replay should be acknowledged");
    assert_eq!(
        jetstream::inspect_recent_messages(client.clone(), stream.clone(), 20)
            .await
            .expect("replayed message should be inspectable")
            .len(),
        2
    );

    jetstream::purge_stream(client.clone(), stream.clone())
        .await
        .expect("stream messages should be purged");
    assert!(
        jetstream::inspect_recent_messages(client.clone(), stream.clone(), 20)
            .await
            .expect("purged stream should be inspectable")
            .is_empty()
    );
    jetstream::delete_consumer(client.clone(), stream.clone(), consumer)
        .await
        .expect("consumer should be deleted");
    jetstream::delete_stream(client, stream)
        .await
        .expect("stream should be deleted");
}

#[tokio::test]
async fn connection_reports_when_jetstream_is_unavailable() {
    let Ok(url) = std::env::var("NATS_NO_JS_URL") else {
        eprintln!("Skipping non-JetStream integration test: NATS_NO_JS_URL is not set");
        return;
    };
    let (client, status, _) = connection::connect_profile(profile_for(url))
        .await
        .expect("should connect to NATS without JetStream");
    assert!(matches!(status, JetStreamStatus::Unavailable(_)));

    let subject = format!("nats_manager_core.{}", Uuid::new_v4().simple());
    let mut subscriber = client
        .subscribe(subject.clone())
        .await
        .expect("subscription should be created");
    jetstream::publish(client, subject, b"core payload".to_vec(), false)
        .await
        .expect("core NATS publish should succeed without JetStream");
    let message = subscriber
        .next()
        .await
        .expect("published core message should arrive");
    assert_eq!(message.payload.as_ref(), b"core payload");
}
