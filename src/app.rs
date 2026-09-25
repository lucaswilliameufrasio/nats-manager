use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use tokio::runtime::Runtime;
use uuid::Uuid;

use nats_manager::connection::{self, JetStreamStatus};
use nats_manager::profile::{Authentication, ConnectionProfile, ProfileStore, TlsConfig};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum AppPage {
    #[default]
    Connections,
    Overview,
    JetStream,
    Publish,
    Maintenance,
}

enum TlsMaterial {
    CaCertificate,
    ClientCertificate,
    ClientPrivateKey,
}

enum UiEvent {
    Connected(Result<(async_nats::Client, JetStreamStatus, String), String>),
    Streams(Result<Vec<String>, String>),
    StreamDetails(Result<nats_manager::jetstream::StreamDetails, String>),
    Consumers(Result<Vec<String>, String>),
    ConsumerDetails(Result<nats_manager::jetstream::ConsumerDetails, String>),
    Messages(Result<Vec<nats_manager::jetstream::StoredMessage>, String>),
    Operation(Result<String, String>),
}

pub struct NatsManagerApp {
    runtime: Runtime,
    current_page: AppPage,
    server_url: String,
    status: String,
    event_receiver: Receiver<UiEvent>,
    event_sender: Sender<UiEvent>,
    active_client: Option<async_nats::Client>,
    server_info: Option<async_nats::ServerInfo>,
    jetstream_api_prefix: Option<String>,
    jetstream_status: Option<JetStreamStatus>,
    profile_store: Option<ProfileStore>,
    profiles: Vec<ConnectionProfile>,
    profile_name: String,
    username: String,
    password: String,
    use_password_auth: bool,
    tls_ca_certificate_key: Option<String>,
    tls_client_certificate_key: Option<String>,
    tls_client_private_key_key: Option<String>,
    streams: Vec<String>,
    selected_stream: Option<String>,
    consumers: Vec<String>,
    selected_consumer: Option<String>,
    stream_name_confirmation: String,
    consumer_name_confirmation: String,
    new_stream_name: String,
    new_stream_subject: String,
    new_consumer_name: String,
    max_deliver_input: String,
    ack_wait_input: String,
    jetstream_message: String,
    stream_details: Option<nats_manager::jetstream::StreamDetails>,
    messages: Vec<nats_manager::jetstream::StoredMessage>,
    selected_message: Option<u64>,
    publish_subject: String,
    publish_payload: String,
    publish_via_jetstream: bool,
}

impl NatsManagerApp {
    pub fn new(runtime: Runtime) -> Self {
        let (event_sender, event_receiver) = mpsc::channel();
        let profile_store = ProfileStore::default_location().ok();
        let profiles = profile_store
            .as_ref()
            .and_then(|store| store.load().ok())
            .unwrap_or_default();

        Self {
            runtime,
            current_page: AppPage::Connections,
            server_url: "nats://127.0.0.1:4222".to_owned(),
            status: "Not connected".to_owned(),
            event_receiver,
            event_sender,
            active_client: None,
            server_info: None,
            jetstream_api_prefix: None,
            jetstream_status: None,
            profile_store,
            profiles,
            profile_name: String::new(),
            username: String::new(),
            password: String::new(),
            use_password_auth: false,
            tls_ca_certificate_key: None,
            tls_client_certificate_key: None,
            tls_client_private_key_key: None,
            streams: Vec::new(),
            selected_stream: None,
            consumers: Vec::new(),
            selected_consumer: None,
            stream_name_confirmation: String::new(),
            consumer_name_confirmation: String::new(),
            new_stream_name: String::new(),
            new_stream_subject: String::new(),
            new_consumer_name: String::new(),
            max_deliver_input: String::new(),
            ack_wait_input: String::new(),
            jetstream_message: String::new(),
            stream_details: None,
            messages: Vec::new(),
            selected_message: None,
            publish_subject: String::new(),
            publish_payload: String::new(),
            publish_via_jetstream: false,
        }
    }

    fn save_profile(&mut self) {
        let name = self.profile_name.trim().to_owned();
        let servers = self
            .server_url
            .split(',')
            .map(str::trim)
            .filter(|server| !server.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();

        if name.is_empty() || servers.is_empty() {
            self.status = "Profile name and at least one server URL are required".to_owned();
            return;
        }

        let authentication = if self.use_password_auth {
            if self.username.trim().is_empty() || self.password.is_empty() {
                self.status = "Username and password are required".to_owned();
                return;
            }

            Authentication::UserPassword {
                username: self.username.trim().to_owned(),
                credential_key: format!("{}:password", Uuid::new_v4()),
            }
        } else {
            Authentication::None
        };

        let Some(store) = &self.profile_store else {
            self.status = "Could not locate the application config directory".to_owned();
            return;
        };

        let mut profile = ConnectionProfile::new(name, servers, authentication);
        profile.tls = self.tls_config();
        if self.use_password_auth {
            let Authentication::UserPassword { credential_key, .. } = &profile.authentication
            else {
                unreachable!("password authentication was selected")
            };
            if let Err(error) = nats_manager::credentials::store(credential_key, &self.password) {
                self.status = format!("Could not save password to system keychain: {error}");
                return;
            }
            self.password.clear();
        }

        let mut profiles = self.profiles.clone();
        profiles.push(profile.clone());
        match store.save(&profiles) {
            Ok(()) => {
                self.profiles = profiles;
                self.status = "Connection profile saved".to_owned();
            }
            Err(error) => {
                if let Authentication::UserPassword { credential_key, .. } = profile.authentication
                {
                    let _ = nats_manager::credentials::delete(&credential_key);
                }
                self.status = format!("Could not save profile: {error}");
            }
        }
    }

    fn import_credentials_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NATS credentials", &["creds"])
            .pick_file()
        else {
            return;
        };

        let Some(store) = &self.profile_store else {
            self.status = "Could not locate the application config directory".to_owned();
            return;
        };
        let name = if self.profile_name.trim().is_empty() {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Imported NATS credentials")
                .to_owned()
        } else {
            self.profile_name.trim().to_owned()
        };
        let servers = self
            .server_url
            .split(',')
            .map(str::trim)
            .filter(|server| !server.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();

        if servers.is_empty() {
            self.status = "Enter at least one server URL before importing credentials".to_owned();
            return;
        }

        let secret = match std::fs::read_to_string(&path) {
            Ok(secret) if !secret.trim().is_empty() => secret,
            Ok(_) => {
                self.status = "The selected credentials file is empty".to_owned();
                return;
            }
            Err(error) => {
                self.status = format!("Could not read credentials file: {error}");
                return;
            }
        };

        let credential_key = format!("{}:creds", Uuid::new_v4());
        if let Err(error) = nats_manager::credentials::store(&credential_key, &secret) {
            self.status = format!("Could not save credentials to system keychain: {error}");
            return;
        }

        let mut profile = ConnectionProfile::new(
            name,
            servers,
            Authentication::CredentialsFile {
                credential_key: credential_key.clone(),
            },
        );
        profile.tls = self.tls_config();
        let mut profiles = self.profiles.clone();
        profiles.push(profile);
        match store.save(&profiles) {
            Ok(()) => {
                self.profiles = profiles;
                self.status = "Credentials imported into system keychain".to_owned();
            }
            Err(error) => {
                let _ = nats_manager::credentials::delete(&credential_key);
                self.status = format!("Could not save imported profile: {error}");
            }
        }
    }

    fn tls_config(&self) -> Option<TlsConfig> {
        if self.tls_ca_certificate_key.is_none()
            && self.tls_client_certificate_key.is_none()
            && self.tls_client_private_key_key.is_none()
        {
            return None;
        }
        Some(TlsConfig {
            ca_certificate_key: self.tls_ca_certificate_key.clone(),
            client_certificate_key: self.tls_client_certificate_key.clone(),
            client_private_key_key: self.tls_client_private_key_key.clone(),
            tls_first: false,
        })
    }

    fn import_tls_material(&mut self, material: TlsMaterial) {
        let (title, key_slot) = match material {
            TlsMaterial::CaCertificate => {
                ("Import CA certificate", &mut self.tls_ca_certificate_key)
            }
            TlsMaterial::ClientCertificate => (
                "Import client certificate",
                &mut self.tls_client_certificate_key,
            ),
            TlsMaterial::ClientPrivateKey => (
                "Import client private key",
                &mut self.tls_client_private_key_key,
            ),
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title(title)
            .add_filter("PEM files", &["pem", "crt", "cer", "key"])
            .pick_file()
        else {
            return;
        };
        let pem = match std::fs::read_to_string(path) {
            Ok(pem) if !pem.trim().is_empty() => pem,
            Ok(_) => {
                self.status = "The selected PEM file is empty".to_owned();
                return;
            }
            Err(error) => {
                self.status = format!("Could not read PEM file: {error}");
                return;
            }
        };
        let key = format!("{}:tls", Uuid::new_v4());
        if let Err(error) = nats_manager::credentials::store(&key, &pem) {
            self.status = format!("Could not store TLS material in system keychain: {error}");
            return;
        }
        *key_slot = Some(key);
        self.status =
            "TLS material stored in system keychain; save the profile to keep it".to_owned();
    }

    fn import_nats_context(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import NATS CLI context")
            .add_filter("NATS context JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        let Some(store) = &self.profile_store else {
            self.status = "Could not locate the application config directory".to_owned();
            return;
        };
        let imported = match nats_manager::context_import::import_context(&path) {
            Ok(imported) => imported,
            Err(error) => {
                self.status = format!("Could not import NATS context: {error}");
                return;
            }
        };
        let mut profiles = self.profiles.clone();
        profiles.push(imported.profile);
        match store.save(&profiles) {
            Ok(()) => {
                self.profiles = profiles;
                self.status =
                    "NATS CLI context imported; secrets stored in system keychain".to_owned();
            }
            Err(error) => {
                for key in imported.credential_keys {
                    let _ = nats_manager::credentials::delete(&key);
                }
                self.status = format!("Could not save imported context: {error}");
            }
        }
    }

    fn connect(&mut self) {
        let url = self.server_url.trim().to_owned();

        if url.is_empty() {
            self.status = "Enter a NATS server URL".to_owned();
            return;
        }

        self.status = format!("Connecting to {url}…");
        let profile = ConnectionProfile::new(
            "Temporary connection".to_owned(),
            url.split(',').map(str::trim).map(str::to_owned).collect(),
            Authentication::None,
        );
        self.spawn_connection(profile);
    }

    fn connect_saved_profile(&mut self, profile: ConnectionProfile) {
        self.status = format!("Connecting to profile {}…", profile.name);
        self.spawn_connection(profile);
    }

    fn spawn_connection(&mut self, profile: ConnectionProfile) {
        self.active_client = None;
        self.server_info = None;
        self.jetstream_api_prefix = profile.jetstream_api_prefix.clone();
        self.jetstream_status = None;
        let sender = self.event_sender.clone();
        self.runtime.spawn(async move {
            let _ = sender.send(UiEvent::Connected(
                connection::connect_profile(profile).await,
            ));
        });
    }

    fn refresh_streams(&mut self) {
        let Some(client) = self.active_client.clone() else {
            return;
        };
        let sender = self.event_sender.clone();
        let api_prefix = self.jetstream_api_prefix.clone();
        self.runtime.spawn(async move {
            let _ = sender.send(UiEvent::Streams(
                nats_manager::jetstream::list_streams(client, api_prefix).await,
            ));
        });
    }

    fn refresh_consumers(&mut self) {
        let (Some(client), Some(stream_name)) =
            (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        let sender = self.event_sender.clone();
        let api_prefix = self.jetstream_api_prefix.clone();
        self.runtime.spawn(async move {
            let result =
                nats_manager::jetstream::list_consumers(client, stream_name, api_prefix).await;
            let _ = sender.send(UiEvent::Consumers(result));
        });
    }

    fn refresh_stream_details(&mut self) {
        let (Some(client), Some(name)) = (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        let sender = self.event_sender.clone();
        let api_prefix = self.jetstream_api_prefix.clone();
        self.runtime.spawn(async move {
            let result = nats_manager::jetstream::describe_stream(client, name, api_prefix).await;
            let _ = sender.send(UiEvent::StreamDetails(result));
        });
    }

    fn refresh_consumer_details(&mut self) {
        let (Some(client), Some(stream_name), Some(consumer_name)) = (
            self.active_client.clone(),
            self.selected_stream.clone(),
            self.selected_consumer.clone(),
        ) else {
            return;
        };
        let sender = self.event_sender.clone();
        let api_prefix = self.jetstream_api_prefix.clone();
        self.runtime.spawn(async move {
            let result = nats_manager::jetstream::describe_consumer(
                client,
                stream_name,
                consumer_name,
                api_prefix,
            )
            .await;
            let _ = sender.send(UiEvent::ConsumerDetails(result));
        });
    }

    fn run_operation<F>(&mut self, operation: F)
    where
        F: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        let sender = self.event_sender.clone();
        self.runtime.spawn(async move {
            let result = operation.await.map(|()| "Operation completed".to_owned());
            let _ = sender.send(UiEvent::Operation(result));
        });
    }

    fn run_reported_operation<F>(&mut self, operation: F)
    where
        F: std::future::Future<Output = Result<String, String>> + Send + 'static,
    {
        let sender = self.event_sender.clone();
        self.runtime.spawn(async move {
            let _ = sender.send(UiEvent::Operation(operation.await));
        });
    }

    fn create_stream(&mut self) {
        let (Some(client), name, subject) = (
            self.active_client.clone(),
            self.new_stream_name.trim().to_owned(),
            self.new_stream_subject.trim().to_owned(),
        ) else {
            return;
        };
        if name.is_empty() || subject.is_empty() {
            self.jetstream_message = "Stream name and subject are required".to_owned();
            return;
        }
        self.jetstream_message = format!("Creating stream {name}…");
        let sender = self.event_sender.clone();
        let api_prefix = self.jetstream_api_prefix.clone();
        self.runtime.spawn(async move {
            let result = nats_manager::jetstream::create_stream(client, name, subject, api_prefix)
                .await
                .map(|()| "Stream created".to_owned());
            let _ = sender.send(UiEvent::Operation(result));
        });
    }

    fn update_selected_stream_subject(&mut self) {
        let (Some(client), Some(name)) = (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        let subject = self.new_stream_subject.trim().to_owned();
        if subject.is_empty() {
            self.jetstream_message = "Stream subject is required".to_owned();
            return;
        }
        self.run_operation(nats_manager::jetstream::update_stream_subject(
            client,
            name,
            subject,
            self.jetstream_api_prefix.clone(),
        ));
    }

    fn delete_selected_stream(&mut self) {
        let (Some(client), Some(name)) = (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        if !nats_manager::safety::confirms_resource_name(&name, &self.stream_name_confirmation) {
            self.jetstream_message = "Type the exact stream name to confirm deletion".to_owned();
            return;
        }
        self.stream_name_confirmation.clear();
        self.jetstream_message = format!("Deleting stream {name}…");
        self.run_operation(nats_manager::jetstream::delete_stream(
            client,
            name,
            self.jetstream_api_prefix.clone(),
        ));
    }

    fn create_consumer(&mut self) {
        let (Some(client), Some(stream_name)) =
            (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        let name = self.new_consumer_name.trim().to_owned();
        if name.is_empty() {
            self.jetstream_message = "Consumer name is required".to_owned();
            return;
        }
        self.jetstream_message = format!("Creating consumer {name}…");
        self.run_operation(nats_manager::jetstream::create_pull_consumer(
            client,
            stream_name,
            name,
            self.jetstream_api_prefix.clone(),
        ));
    }

    fn update_selected_consumer_limits(&mut self) {
        let (Some(client), Some(stream_name), Some(consumer_name)) = (
            self.active_client.clone(),
            self.selected_stream.clone(),
            self.selected_consumer.clone(),
        ) else {
            return;
        };
        let max_deliver = match self.max_deliver_input.trim().parse::<i64>() {
            Ok(value) if value >= 0 => value,
            _ => {
                self.jetstream_message = "Max deliveries must be a non-negative integer".to_owned();
                return;
            }
        };
        let ack_wait_seconds = match self.ack_wait_input.trim().parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                self.jetstream_message = "Ack wait must be a non-negative integer".to_owned();
                return;
            }
        };
        self.run_operation(nats_manager::jetstream::update_consumer_limits(
            client,
            stream_name,
            consumer_name,
            max_deliver,
            ack_wait_seconds,
            self.jetstream_api_prefix.clone(),
        ));
    }

    fn delete_selected_consumer(&mut self) {
        let (Some(client), Some(stream_name), Some(consumer_name)) = (
            self.active_client.clone(),
            self.selected_stream.clone(),
            self.selected_consumer.clone(),
        ) else {
            return;
        };
        if !nats_manager::safety::confirms_resource_name(
            &consumer_name,
            &self.consumer_name_confirmation,
        ) {
            self.jetstream_message = "Type the exact consumer name to confirm deletion".to_owned();
            return;
        }
        self.consumer_name_confirmation.clear();
        self.jetstream_message = format!("Deleting consumer {consumer_name}…");
        self.run_operation(nats_manager::jetstream::delete_consumer(
            client,
            stream_name,
            consumer_name,
            self.jetstream_api_prefix.clone(),
        ));
    }

    fn inspect_recent_messages(&mut self) {
        let (Some(client), Some(stream_name)) =
            (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        let sender = self.event_sender.clone();
        let api_prefix = self.jetstream_api_prefix.clone();
        self.jetstream_message = format!("Loading recent messages from {stream_name}…");
        self.runtime.spawn(async move {
            let result = nats_manager::jetstream::inspect_recent_messages(
                client,
                stream_name,
                20,
                api_prefix,
            )
            .await;
            let _ = sender.send(UiEvent::Messages(result));
        });
    }

    fn publish_message(&mut self) {
        let Some(client) = self.active_client.clone() else {
            return;
        };
        let subject = self.publish_subject.trim().to_owned();
        if subject.is_empty() {
            self.jetstream_message = "Message subject is required".to_owned();
            return;
        }
        let payload = self.publish_payload.as_bytes().to_vec();
        let jetstream = self.publish_via_jetstream
            && matches!(
                &self.jetstream_status,
                Some(JetStreamStatus::Enabled { .. })
            );
        let sender = self.event_sender.clone();
        let api_prefix = self.jetstream_api_prefix.clone();
        self.jetstream_message = format!("Publishing to {subject}…");
        self.runtime.spawn(async move {
            let result = nats_manager::jetstream::publish(
                client,
                subject,
                payload,
                async_nats::HeaderMap::new(),
                jetstream,
                api_prefix,
            )
            .await;
            let _ = sender.send(UiEvent::Operation(result));
        });
        self.publish_payload.clear();
    }

    fn replay_selected_message(&mut self) {
        let (Some(client), Some(sequence)) = (self.active_client.clone(), self.selected_message)
        else {
            return;
        };
        let Some(message) = self
            .messages
            .iter()
            .find(|message| message.sequence == sequence)
        else {
            return;
        };
        self.run_reported_operation(nats_manager::jetstream::replay_message(
            client,
            message.subject.clone(),
            message.payload.clone(),
            message.headers.clone(),
            self.jetstream_api_prefix.clone(),
        ));
    }

    fn purge_selected_stream(&mut self) {
        let (Some(client), Some(stream_name)) =
            (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        if !nats_manager::safety::confirms_resource_name(
            &stream_name,
            &self.stream_name_confirmation,
        ) {
            self.jetstream_message = "Type the exact stream name to confirm purge".to_owned();
            return;
        }
        self.stream_name_confirmation.clear();
        self.run_reported_operation(nats_manager::jetstream::purge_stream(
            client,
            stream_name,
            self.jetstream_api_prefix.clone(),
        ));
    }
}

impl eframe::App for NatsManagerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        for _ in 0..16 {
            let event = match self.event_receiver.try_recv() {
                Ok(event) => event,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = "Background task channel stopped unexpectedly".to_owned();
                    break;
                }
            };

            match event {
                UiEvent::Connected(Ok((client, jetstream_status, target))) => {
                    let info = client.server_info();
                    self.status = format!("Connected to {target} (server {})", info.version);
                    self.server_info = Some(info);
                    self.active_client = Some(client);
                    self.jetstream_status = Some(jetstream_status);
                    self.current_page = AppPage::Overview;
                    if matches!(
                        &self.jetstream_status,
                        Some(JetStreamStatus::Enabled { .. })
                    ) {
                        self.refresh_streams();
                    }
                }
                UiEvent::Connected(Err(error)) => {
                    self.status = format!("Connection failed: {error}");
                }
                UiEvent::Streams(Ok(streams)) => {
                    self.streams = streams;
                    if !self
                        .selected_stream
                        .as_ref()
                        .is_some_and(|selected| self.streams.contains(selected))
                    {
                        self.selected_stream = self.streams.first().cloned();
                    }
                    self.refresh_stream_details();
                    self.refresh_consumers();
                }
                UiEvent::Streams(Err(error)) => {
                    self.jetstream_message = format!("Could not list streams: {error}");
                }
                UiEvent::StreamDetails(Ok(details)) => {
                    self.stream_details = Some(details);
                }
                UiEvent::StreamDetails(Err(error)) => {
                    self.jetstream_message = format!("Could not load stream details: {error}");
                }
                UiEvent::Consumers(Ok(consumers)) => {
                    let previous = self.selected_consumer.clone();
                    self.consumers = consumers;
                    if !self
                        .selected_consumer
                        .as_ref()
                        .is_some_and(|selected| self.consumers.contains(selected))
                    {
                        self.selected_consumer = self.consumers.first().cloned();
                    }
                    if previous != self.selected_consumer {
                        self.refresh_consumer_details();
                    }
                }
                UiEvent::Consumers(Err(error)) => {
                    self.jetstream_message = format!("Could not list consumers: {error}");
                }
                UiEvent::ConsumerDetails(Ok(details)) => {
                    self.max_deliver_input = details.max_deliver.to_string();
                    self.ack_wait_input = details.ack_wait_seconds.to_string();
                }
                UiEvent::ConsumerDetails(Err(error)) => {
                    self.jetstream_message = format!("Could not load consumer details: {error}");
                }
                UiEvent::Messages(Ok(messages)) => {
                    self.messages = messages;
                    self.selected_message = self.messages.last().map(|message| message.sequence);
                    self.jetstream_message =
                        format!("Loaded {} recent message(s)", self.messages.len());
                }
                UiEvent::Messages(Err(error)) => {
                    self.jetstream_message = format!("Could not inspect messages: {error}");
                }
                UiEvent::Operation(Ok(message)) => {
                    self.jetstream_message = message;
                    self.refresh_streams();
                }
                UiEvent::Operation(Err(error)) => {
                    self.jetstream_message = format!("Operation failed: {error}");
                }
            }
        }

        egui::Panel::left("main_navigation")
            .resizable(false)
            .default_size(184.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("NATS Manager");
                ui.small(format!("v{}", env!("CARGO_PKG_VERSION")));
                ui.add_space(20.0);
                for (page, label) in [
                    (AppPage::Connections, "Connections"),
                    (AppPage::Overview, "Overview"),
                    (AppPage::JetStream, "JetStream"),
                    (AppPage::Publish, "Publish"),
                    (AppPage::Maintenance, "Maintenance"),
                ] {
                    if ui
                        .selectable_label(self.current_page == page, label)
                        .clicked()
                    {
                        self.current_page = page;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.separator();
                    ui.label(if self.active_client.is_some() {
                        "● Connected"
                    } else {
                        "○ Disconnected"
                    });
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(match self.current_page {
                    AppPage::Connections => "Connections",
                    AppPage::Overview => "Cluster overview",
                    AppPage::JetStream => "JetStream",
                    AppPage::Publish => "Publish messages",
                    AppPage::Maintenance => "Maintenance",
                });
                ui.add_space(12.0);

                if self.current_page == AppPage::Connections {
                    ui.label("Connect to existing NATS clusters and manage saved profiles.");
                    ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Server URL");
            ui.text_edit_singleline(&mut self.server_url);
            if ui.button("Connect").clicked() {
                self.connect();
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.heading("Save connection profile");
        ui.horizontal(|ui| {
            ui.label("Profile name");
            ui.text_edit_singleline(&mut self.profile_name);
        });
        ui.checkbox(&mut self.use_password_auth, "Use username and password");
        if self.use_password_auth {
            ui.horizontal(|ui| {
                ui.label("Username");
                ui.text_edit_singleline(&mut self.username);
            });
            ui.horizontal(|ui| {
                ui.label("Password");
                ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
            });
        }
        if ui.button("Save profile securely").clicked() {
            self.save_profile();
        }
        if ui
            .button("Import .creds file into system keychain")
            .clicked()
        {
            self.import_credentials_file();
        }
        if ui.button("Import NATS CLI context JSON").clicked() {
            self.import_nats_context();
        }
        ui.separator();
        ui.label("TLS / mTLS (imported PEM files are stored in the system keychain)");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Import CA certificate").clicked() {
                self.import_tls_material(TlsMaterial::CaCertificate);
            }
            if ui.button("Import client certificate").clicked() {
                self.import_tls_material(TlsMaterial::ClientCertificate);
            }
            if ui.button("Import client private key").clicked() {
                self.import_tls_material(TlsMaterial::ClientPrivateKey);
            }
        });
        ui.label(format!(
            "CA: {} · client certificate: {} · private key: {}",
            self.tls_ca_certificate_key.is_some(),
            self.tls_client_certificate_key.is_some(),
            self.tls_client_private_key_key.is_some()
        ));

        if !self.profiles.is_empty() {
            ui.add_space(12.0);
            ui.heading("Saved profiles");
            for profile in self.profiles.clone() {
                ui.horizontal(|ui| {
                    ui.label(format!("{} — {}", profile.name, profile.servers.join(", ")));
                    if ui.button("Connect").clicked() {
                        self.connect_saved_profile(profile);
                    }
                });
            }
        }

                }

                if self.current_page == AppPage::Overview {
        ui.add_space(8.0);
        ui.label(&self.status);
        if let Some(info) = &self.server_info {
            egui::Grid::new("nats_server_info")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Server");
                    ui.label(format!("{} ({})", info.server_name, info.server_id));
                    ui.end_row();
                    ui.label("Address");
                    ui.label(format!("{}:{}", info.host, info.port));
                    ui.end_row();
                    ui.label("Cluster / domain");
                    ui.label(format!(
                        "{} / {}",
                        info.cluster.as_deref().unwrap_or("—"),
                        info.domain.as_deref().unwrap_or("—")
                    ));
                    ui.end_row();
                    ui.label("Max payload / client IP");
                    ui.label(format!("{} B / {}", info.max_payload, info.client_ip));
                    ui.end_row();
                    ui.label("Advertised servers");
                    ui.label(if info.connect_urls.is_empty() {
                        "—".to_owned()
                    } else {
                        info.connect_urls.join(", ")
                    });
                    ui.end_row();
                });
        }
        if let Some(status) = &self.jetstream_status {
            match status {
                JetStreamStatus::Enabled {
                    streams,
                    consumers,
                    memory_bytes,
                    storage_bytes,
                    domain,
                } => {
                    ui.label(format!(
                        "JetStream enabled · {streams} stream(s) · {consumers} consumer(s) · memory {memory_bytes} B · storage {storage_bytes} B{}",
                        domain.as_ref().map(|value| format!(" · domain {value}")).unwrap_or_default()
                    ));
                }
                JetStreamStatus::Unavailable(reason) => {
                    ui.label(format!("JetStream: unavailable ({reason})"));
                }
            }
        }
        ui.add_space(16.0);
        if self.active_client.is_none() {
            ui.weak("Cluster details will appear after connecting.");
        }
                }

        if matches!(
            &self.jetstream_status,
            Some(JetStreamStatus::Enabled { .. })
        ) && self.current_page == AppPage::JetStream
        {
            ui.add_space(16.0);
            ui.separator();
            ui.heading("JetStream");
            ui.horizontal(|ui| {
                if ui.button("Refresh streams").clicked() {
                    self.refresh_streams();
                }
                ui.label(format!("{} stream(s)", self.streams.len()));
            });
            ui.horizontal(|ui| {
                ui.label("Stream name");
                ui.text_edit_singleline(&mut self.new_stream_name);
                ui.label("Subject");
                ui.text_edit_singleline(&mut self.new_stream_subject);
                if ui.button("Create stream").clicked() {
                    self.create_stream();
                }
            });

            let mut selected_stream_changed = false;
            ui.horizontal_wrapped(|ui| {
                for stream in self.streams.clone() {
                    if ui
                        .selectable_label(self.selected_stream.as_ref() == Some(&stream), &stream)
                        .clicked()
                    {
                        self.selected_stream = Some(stream);
                        self.selected_consumer = None;
                        self.consumers.clear();
                        self.stream_details = None;
                        selected_stream_changed = true;
                    }
                }
            });
            if selected_stream_changed {
                self.refresh_stream_details();
                self.refresh_consumers();
            }

            if let Some(stream) = self.selected_stream.clone() {
                ui.label(format!("Selected stream: {stream}"));
                if let Some(details) = &self.stream_details {
                    ui.label(format!(
                        "Subjects: {} · {} message(s) · {} B · {} consumer(s)",
                        details.subjects.join(", "),
                        details.messages,
                        details.bytes,
                        details.consumers
                    ));
                }
                ui.horizontal(|ui| {
                    ui.label("New subject filter");
                    ui.text_edit_singleline(&mut self.new_stream_subject);
                    if ui.button("Update stream subject").clicked() {
                        self.update_selected_stream_subject();
                    }
                });
                if ui.button("Inspect recent messages").clicked() {
                    self.inspect_recent_messages();
                }

                ui.separator();
                ui.heading("Consumers");
                ui.horizontal(|ui| {
                    ui.label("New durable pull consumer");
                    ui.text_edit_singleline(&mut self.new_consumer_name);
                    if ui.button("Create consumer").clicked() {
                        self.create_consumer();
                    }
                });
                let mut selected_consumer_changed = false;
                for consumer in self.consumers.clone() {
                    if ui
                        .selectable_label(
                            self.selected_consumer.as_ref() == Some(&consumer),
                            &consumer,
                        )
                        .clicked()
                    {
                        self.selected_consumer = Some(consumer);
                        selected_consumer_changed = true;
                    }
                }
                if selected_consumer_changed {
                    self.refresh_consumer_details();
                }
                if let Some(consumer) = self.selected_consumer.clone() {
                    ui.label(format!("Selected consumer: {consumer}"));
                    ui.horizontal(|ui| {
                        ui.label("Max deliveries");
                        ui.text_edit_singleline(&mut self.max_deliver_input);
                        ui.label("Ack wait (seconds)");
                        ui.text_edit_singleline(&mut self.ack_wait_input);
                        if ui.button("Update consumer limits").clicked() {
                            self.update_selected_consumer_limits();
                        }
                    });
                }

                ui.separator();
                ui.heading("Recent messages");
                for message in self.messages.clone() {
                    let preview = String::from_utf8_lossy(&message.payload);
                    let preview = preview.chars().take(160).collect::<String>();
                    let headers = message
                        .headers
                        .iter()
                        .map(|(name, values)| {
                            format!(
                                "{}: {}",
                                name,
                                values
                                    .iter()
                                    .map(|value| value.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ");
                    ui.selectable_value(
                        &mut self.selected_message,
                        Some(message.sequence),
                        format!(
                            "#{} {} — {}{}",
                            message.sequence,
                            message.subject,
                            preview,
                            if headers.is_empty() {
                                String::new()
                            } else {
                                format!(" · {headers}")
                            }
                        ),
                    );
                }
                if self.selected_message.is_some() && ui.button("Replay selected message").clicked()
                {
                    self.replay_selected_message();
                }
            }
            ui.label(&self.jetstream_message);
        }

        if self.current_page == AppPage::Maintenance {
            ui.label("Destructive actions require typing the exact resource name.");
            ui.add_space(8.0);
            if self.active_client.is_none() {
                ui.weak("Connect to a cluster before managing resources.");
            } else if let Some(JetStreamStatus::Unavailable(reason)) = &self.jetstream_status {
                ui.weak(format!("JetStream is unavailable: {reason}"));
            } else if matches!(
                &self.jetstream_status,
                Some(JetStreamStatus::Enabled { .. })
            ) {
                    ui.horizontal(|ui| {
                        ui.strong("Selected stream");
                        ui.label(self.selected_stream.as_deref().unwrap_or("None"));
                    });
                    if let Some(stream) = self.selected_stream.clone() {
                        ui.label("Type the stream name to delete it or purge its messages.");
                        ui.text_edit_singleline(&mut self.stream_name_confirmation);
                        ui.horizontal(|ui| {
                            if ui.button("Delete stream…").clicked() {
                                self.delete_selected_stream();
                            }
                            if ui.button("Purge stream messages…").clicked() {
                                self.purge_selected_stream();
                            }
                        });
                        ui.small(format!("Selected: {stream}"));
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.strong("Selected consumer");
                        ui.label(self.selected_consumer.as_deref().unwrap_or("None"));
                    });
                    if let Some(consumer) = self.selected_consumer.clone() {
                        ui.label("Type the consumer name to delete it.");
                        ui.text_edit_singleline(&mut self.consumer_name_confirmation);
                        if ui.button("Delete consumer…").clicked() {
                            self.delete_selected_consumer();
                        }
                        ui.small(format!("Selected: {consumer}"));
                    }
            } else {
                ui.weak("Connect to a cluster before managing resources.");
            }
            ui.add_space(8.0);
            ui.label(&self.jetstream_message);
        }

        if self.active_client.is_some() && self.current_page == AppPage::Publish {
            ui.add_space(12.0);
            ui.separator();
            ui.heading("Publish message");
            if matches!(
                &self.jetstream_status,
                Some(JetStreamStatus::Enabled { .. })
            ) {
                ui.checkbox(
                    &mut self.publish_via_jetstream,
                    "Publish through JetStream (requires a matching stream)",
                );
            }
            ui.horizontal(|ui| {
                ui.label("Subject");
                ui.text_edit_singleline(&mut self.publish_subject);
                if ui.button("Publish").clicked() {
                    self.publish_message();
                }
            });
            ui.text_edit_multiline(&mut self.publish_payload);
        }
        if self.current_page == AppPage::Publish && self.active_client.is_none() {
            ui.weak("Connect to a cluster before publishing messages.");
        }
        if self.current_page == AppPage::JetStream
            && !matches!(
                &self.jetstream_status,
                Some(JetStreamStatus::Enabled { .. })
            )
        {
            ui.weak("Connect to a JetStream-enabled cluster to manage streams and consumers.");
        }
            });
        });

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
}
