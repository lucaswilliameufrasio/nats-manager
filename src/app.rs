use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::profile::{Authentication, ConnectionProfile, ProfileStore};

pub struct NatsManagerApp {
    runtime: Runtime,
    server_url: String,
    status: String,
    connection_result: Option<Receiver<Result<async_nats::Client, String>>>,
    connection_sender: Sender<Result<async_nats::Client, String>>,
    active_client: Option<async_nats::Client>,
    profile_store: Option<ProfileStore>,
    profiles: Vec<ConnectionProfile>,
    profile_name: String,
    username: String,
    password: String,
    use_password_auth: bool,
}

impl NatsManagerApp {
    pub fn new(runtime: Runtime) -> Self {
        let (connection_sender, connection_result) = mpsc::channel();
        let profile_store = ProfileStore::default_location().ok();
        let profiles = profile_store
            .as_ref()
            .and_then(|store| store.load().ok())
            .unwrap_or_default();

        Self {
            runtime,
            server_url: "nats://127.0.0.1:4222".to_owned(),
            status: "Not connected".to_owned(),
            connection_result: Some(connection_result),
            connection_sender,
            active_client: None,
            profile_store,
            profiles,
            profile_name: String::new(),
            username: String::new(),
            password: String::new(),
            use_password_auth: false,
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
        let sender = self.connection_sender.clone();

        if url.is_empty() {
            self.status = "Enter a NATS server URL".to_owned();
            return;
        }

        self.status = format!("Connecting to {url}…");
        self.runtime.spawn(async move {
            let result = async_nats::connect(&url)
                .await
                .map_err(|error| error.to_string());

            let _ = sender.send(result);
        });
    }
}

impl eframe::App for NatsManagerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(receiver) = &self.connection_result {
            match receiver.try_recv() {
                Ok(Ok(client)) => {
                    let info = client.server_info();
                    self.status =
                        format!("Connected to {} (server {})", self.server_url, info.version);
                    self.active_client = Some(client);
                }
                Ok(Err(error)) => self.status = format!("Connection failed: {error}"),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status = "Connection task stopped unexpectedly".to_owned();
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
            for profile in &self.profiles {
                ui.label(format!("{} — {}", profile.name, profile.servers.join(", ")));
            }
        }

        ui.add_space(8.0);
        ui.label(&self.status);
        ui.add_space(16.0);
        ui.weak("JetStream and cluster details will appear after connecting.");

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
}
