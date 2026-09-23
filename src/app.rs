use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::connection::{self, JetStreamStatus};
use crate::profile::{Authentication, ConnectionProfile, ProfileStore};

enum UiEvent {
    Connected(Result<(async_nats::Client, JetStreamStatus, String), String>),
    Streams(Result<Vec<String>, String>),
    Consumers(Result<Vec<String>, String>),
    Operation(Result<String, String>),
}

pub struct NatsManagerApp {
    runtime: Runtime,
    server_url: String,
    status: String,
    event_receiver: Receiver<UiEvent>,
    event_sender: Sender<UiEvent>,
    active_client: Option<async_nats::Client>,
    jetstream_status: Option<JetStreamStatus>,
    profile_store: Option<ProfileStore>,
    profiles: Vec<ConnectionProfile>,
    profile_name: String,
    username: String,
    password: String,
    use_password_auth: bool,
    streams: Vec<String>,
    selected_stream: Option<String>,
    consumers: Vec<String>,
    selected_consumer: Option<String>,
    resource_name_confirmation: String,
    new_stream_name: String,
    new_stream_subject: String,
    new_consumer_name: String,
    jetstream_message: String,
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
            server_url: "nats://127.0.0.1:4222".to_owned(),
            status: "Not connected".to_owned(),
            event_receiver,
            event_sender,
            active_client: None,
            jetstream_status: None,
            profile_store,
            profiles,
            profile_name: String::new(),
            username: String::new(),
            password: String::new(),
            use_password_auth: false,
            streams: Vec::new(),
            selected_stream: None,
            consumers: Vec::new(),
            selected_consumer: None,
            resource_name_confirmation: String::new(),
            new_stream_name: String::new(),
            new_stream_subject: String::new(),
            new_consumer_name: String::new(),
            jetstream_message: String::new(),
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

        let profile = ConnectionProfile::new(name, servers, authentication);
        if self.use_password_auth {
            let Authentication::UserPassword { credential_key, .. } = &profile.authentication
            else {
                unreachable!("password authentication was selected")
            };
            if let Err(error) = crate::credentials::store(credential_key, &self.password) {
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
                    let _ = crate::credentials::delete(&credential_key);
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
        if let Err(error) = crate::credentials::store(&credential_key, &secret) {
            self.status = format!("Could not save credentials to system keychain: {error}");
            return;
        }

        let profile = ConnectionProfile::new(
            name,
            servers,
            Authentication::CredentialsFile {
                credential_key: credential_key.clone(),
            },
        );
        let mut profiles = self.profiles.clone();
        profiles.push(profile);
        match store.save(&profiles) {
            Ok(()) => {
                self.profiles = profiles;
                self.status = "Credentials imported into system keychain".to_owned();
            }
            Err(error) => {
                let _ = crate::credentials::delete(&credential_key);
                self.status = format!("Could not save imported profile: {error}");
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
        self.runtime.spawn(async move {
            let _ = sender.send(UiEvent::Streams(
                crate::jetstream::list_streams(client).await,
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
        self.runtime.spawn(async move {
            let result = crate::jetstream::list_consumers(client, stream_name).await;
            let _ = sender.send(UiEvent::Consumers(result));
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
        self.runtime.spawn(async move {
            let result = crate::jetstream::create_stream(client, name, subject)
                .await
                .map(|()| "Stream created".to_owned());
            let _ = sender.send(UiEvent::Operation(result));
        });
    }

    fn delete_selected_stream(&mut self) {
        let (Some(client), Some(name)) = (self.active_client.clone(), self.selected_stream.clone())
        else {
            return;
        };
        if self.resource_name_confirmation != name {
            self.jetstream_message = "Type the exact stream name to confirm deletion".to_owned();
            return;
        }
        self.resource_name_confirmation.clear();
        self.jetstream_message = format!("Deleting stream {name}…");
        self.run_operation(crate::jetstream::delete_stream(client, name));
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
        self.run_operation(crate::jetstream::create_pull_consumer(
            client,
            stream_name,
            name,
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
        if self.resource_name_confirmation != consumer_name {
            self.jetstream_message = "Type the exact consumer name to confirm deletion".to_owned();
            return;
        }
        self.resource_name_confirmation.clear();
        self.jetstream_message = format!("Deleting consumer {consumer_name}…");
        self.run_operation(crate::jetstream::delete_consumer(
            client,
            stream_name,
            consumer_name,
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
                    self.active_client = Some(client);
                    self.jetstream_status = Some(jetstream_status);
                    if matches!(&self.jetstream_status, Some(JetStreamStatus::Enabled)) {
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
                    self.refresh_consumers();
                }
                UiEvent::Streams(Err(error)) => {
                    self.jetstream_message = format!("Could not list streams: {error}");
                }
                UiEvent::Consumers(Ok(consumers)) => {
                    self.consumers = consumers;
                    if !self
                        .selected_consumer
                        .as_ref()
                        .is_some_and(|selected| self.consumers.contains(selected))
                    {
                        self.selected_consumer = self.consumers.first().cloned();
                    }
                }
                UiEvent::Consumers(Err(error)) => {
                    self.jetstream_message = format!("Could not list consumers: {error}");
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

        ui.heading("NATS Manager");
        ui.label("Connect to and manage existing NATS clusters.");
        ui.add_space(16.0);

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

        ui.add_space(8.0);
        ui.label(&self.status);
        if let Some(status) = &self.jetstream_status {
            match status {
                JetStreamStatus::Enabled => {
                    ui.label("JetStream: enabled");
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

        if matches!(&self.jetstream_status, Some(JetStreamStatus::Enabled)) {
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
                        selected_stream_changed = true;
                    }
                }
            });
            if selected_stream_changed {
                self.refresh_consumers();
            }

            if let Some(stream) = self.selected_stream.clone() {
                ui.label(format!("Selected stream: {stream}"));
                ui.horizontal(|ui| {
                    ui.label("Type stream name to delete");
                    ui.text_edit_singleline(&mut self.resource_name_confirmation);
                    if ui.button("Delete stream").clicked() {
                        self.delete_selected_stream();
                    }
                });

                ui.separator();
                ui.heading("Consumers");
                ui.horizontal(|ui| {
                    ui.label("New durable pull consumer");
                    ui.text_edit_singleline(&mut self.new_consumer_name);
                    if ui.button("Create consumer").clicked() {
                        self.create_consumer();
                    }
                });
                for consumer in self.consumers.clone() {
                    ui.selectable_value(
                        &mut self.selected_consumer,
                        Some(consumer.clone()),
                        consumer,
                    );
                }
                if let Some(consumer) = self.selected_consumer.clone() {
                    ui.horizontal(|ui| {
                        ui.label(format!("Selected consumer: {consumer}"));
                        if ui.button("Delete consumer").clicked() {
                            self.delete_selected_consumer();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Type consumer name to confirm");
                        ui.text_edit_singleline(&mut self.resource_name_confirmation);
                    });
                }
            }
            ui.label(&self.jetstream_message);
        }

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
}
