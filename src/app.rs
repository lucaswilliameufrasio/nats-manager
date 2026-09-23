use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use tokio::runtime::Runtime;

pub struct NatsManagerApp {
    runtime: Runtime,
    server_url: String,
    status: String,
    connection_result: Option<Receiver<Result<async_nats::Client, String>>>,
    connection_sender: Sender<Result<async_nats::Client, String>>,
    active_client: Option<async_nats::Client>,
}

impl NatsManagerApp {
    pub fn new(runtime: Runtime) -> Self {
        let (connection_sender, connection_result) = mpsc::channel();

        Self {
            runtime,
            server_url: "nats://127.0.0.1:4222".to_owned(),
            status: "Not connected".to_owned(),
            connection_result: Some(connection_result),
            connection_sender,
            active_client: None,
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

        ui.add_space(8.0);
        ui.label(&self.status);
        ui.add_space(16.0);
        ui.weak("JetStream and cluster details will appear after connecting.");

        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
}
