use eframe::egui::{self, Color32, RichText, Stroke, vec2};
use nats_manager::connection::JetStreamStatus;
use nats_manager::profile::{Authentication, ConnectionProfile};

use super::theme;
use super::{AppPage, NatsManagerApp, TlsMaterial};

const CONTENT_WIDTH: f32 = 1180.0;

pub(super) fn show(app: &mut NatsManagerApp, ui: &mut egui::Ui) {
    egui::Panel::left("main_navigation")
        .resizable(false)
        .default_size(218.0)
        .frame(egui::Frame::new().fill(theme::SIDEBAR))
        .show(ui, |ui| show_navigation(app, ui));

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(theme::CANVAS))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("main_content_scroll")
                .show(ui, |ui| {
                    ui.add_space(22.0);
                    ui.vertical_centered_justified(|ui| {
                        ui.set_max_width(CONTENT_WIDTH);
                        show_page_header(app, ui);
                        ui.add_space(22.0);
                        match app.current_page {
                            AppPage::Connections => show_connections(app, ui),
                            AppPage::Overview => show_overview(app, ui),
                            AppPage::JetStream => show_jetstream(app, ui),
                            AppPage::Publish => show_publish(app, ui),
                            AppPage::Maintenance => show_maintenance(app, ui),
                        }
                        ui.add_space(28.0);
                    });
                });
        });
}

fn show_navigation(app: &mut NatsManagerApp, ui: &mut egui::Ui) {
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.add_space(17.0);
        draw_brand_mark(ui, 40.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("NATS").size(18.0).strong().color(theme::TEXT));
            ui.label(
                RichText::new("MANAGER")
                    .size(10.0)
                    .strong()
                    .color(theme::ACCENT),
            );
        });
    });
    ui.add_space(7.0);
    ui.label(
        RichText::new(format!("VERSION {}", env!("CARGO_PKG_VERSION")))
            .small()
            .color(theme::MUTED),
    );
    ui.add_space(30.0);
    ui.add_space(16.0);
    ui.label(
        RichText::new("WORKSPACE")
            .small()
            .strong()
            .color(theme::MUTED),
    );
    ui.add_space(8.0);

    for (page, glyph, label) in [
        (AppPage::Overview, "⌂", "Overview"),
        (AppPage::Connections, "↔", "Connections"),
        (AppPage::JetStream, "▦", "JetStream"),
        (AppPage::Publish, "↑", "Publish"),
        (AppPage::Maintenance, "⚠", "Maintenance"),
    ] {
        if nav_entry(ui, app.current_page == page, glyph, label).clicked() {
            app.current_page = page;
        }
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(16.0);
        card(ui, |ui| {
            if let Some(info) = &app.server_info {
                status_dot(ui, theme::SUCCESS);
                ui.label(
                    RichText::new("CONNECTED")
                        .small()
                        .strong()
                        .color(theme::SUCCESS),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(&info.server_name).strong().color(theme::TEXT));
                ui.label(
                    RichText::new(format!("{}:{}", info.host, info.port))
                        .small()
                        .color(theme::MUTED),
                );
            } else {
                status_dot(ui, theme::MUTED);
                ui.label(
                    RichText::new("DISCONNECTED")
                        .small()
                        .strong()
                        .color(theme::MUTED),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Connect a cluster to get started")
                        .small()
                        .color(theme::MUTED),
                );
            }
        });
        ui.add_space(16.0);
        ui.label(
            RichText::new("NATS CLUSTER WORKSPACE")
                .small()
                .color(theme::MUTED),
        );
    });
}

fn draw_brand_mark(ui: &mut egui::Ui, size: f32) {
    let (response, painter) = ui.allocate_painter(vec2(size, size), egui::Sense::hover());
    let rect = response.rect;
    let center = rect.center();
    let left_top = rect.left_top() + vec2(size * 0.23, size * 0.26);
    let right_top = rect.right_top() + vec2(-size * 0.23, size * 0.26);
    let left_bottom = rect.left_bottom() + vec2(size * 0.23, -size * 0.26);
    let right_bottom = rect.right_bottom() + vec2(-size * 0.23, -size * 0.26);
    let line = Stroke::new(2.3, theme::ACCENT);
    painter.line_segment([left_top, left_bottom], line);
    painter.line_segment([left_top, right_bottom], line);
    painter.line_segment([right_top, right_bottom], line);
    painter.circle_filled(center, size * 0.09, Color32::WHITE);
    for point in [left_top, right_top, left_bottom, right_bottom] {
        painter.circle_filled(point, size * 0.065, theme::ACCENT);
    }
}

fn nav_entry(ui: &mut egui::Ui, selected: bool, glyph: &str, label: &str) -> egui::Response {
    let fill = if selected {
        theme::ACCENT_DARK
    } else {
        theme::SIDEBAR
    };
    let foreground = if selected {
        theme::ACCENT
    } else {
        theme::MUTED
    };
    ui.add_sized(
        [ui.available_width(), 44.0],
        egui::Button::new(
            RichText::new(format!("{glyph}     {label}"))
                .color(if selected { theme::TEXT } else { foreground })
                .strong(),
        )
        .fill(fill)
        .stroke(Stroke::new(
            1.0,
            if selected {
                theme::ACCENT.gamma_multiply(0.35)
            } else {
                fill
            },
        ))
        .corner_radius(9),
    )
}

fn show_page_header(app: &NatsManagerApp, ui: &mut egui::Ui) {
    let (eyebrow, title, subtitle) = match app.current_page {
        AppPage::Connections => (
            "CLUSTER ACCESS",
            "Connections",
            "Connect securely and keep your cluster profiles organized.",
        ),
        AppPage::Overview => (
            "LIVE ENVIRONMENT",
            "Cluster overview",
            "A clear view of server identity, health, and JetStream capacity.",
        ),
        AppPage::JetStream => (
            "STREAMING MESSAGING",
            "JetStream",
            "Explore streams, consumers, and retained messages.",
        ),
        AppPage::Publish => (
            "MESSAGE WORKBENCH",
            "Publish",
            "Compose and send a message to core NATS or JetStream.",
        ),
        AppPage::Maintenance => (
            "ADMINISTRATION",
            "Maintenance",
            "Destructive operations are isolated and require exact-name confirmation.",
        ),
    };

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(eyebrow).small().strong().color(theme::ACCENT));
            ui.add_space(3.0);
            ui.heading(title);
            ui.label(RichText::new(subtitle).color(theme::MUTED));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            if app.active_client.is_some() {
                badge(ui, "●  CONNECTED", theme::SUCCESS, theme::ACCENT_DARK);
            } else {
                badge(ui, "○  NOT CONNECTED", theme::MUTED, theme::SURFACE_RAISED);
            }
        });
    });
}

fn show_connections(app: &mut NatsManagerApp, ui: &mut egui::Ui) {
    card(ui, |ui| {
        section_heading(
            ui,
            "Quick connect",
            "Open a one-time connection without saving credentials.",
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [ui.available_width() * 0.72, 42.0],
                egui::TextEdit::singleline(&mut app.server_url)
                    .hint_text("nats://127.0.0.1:4222 or nats://host-a:4222,host-b:4222")
                    .id_salt("quick-connect-server-url"),
            );
            if primary_button(ui, "Connect").clicked() {
                app.connect();
            }
        });
        feedback_line(ui, &app.status);
    });

    card(ui, |ui| {
        section_heading(
            ui,
            "Saved profiles",
            "Profiles keep secret material in the system credential store.",
        );
        ui.add_space(10.0);
        if app.profiles.is_empty() {
            inline_empty_state(
                ui,
                "No saved profiles yet",
                "Add a profile below or import a NATS CLI context.",
            );
        } else {
            for profile in app.profiles.clone() {
                profile_row(app, ui, profile);
            }
        }
    });

    card(ui, |ui| {
        section_heading(
            ui,
            "Add a profile",
            "Configure authentication and transport security for a reusable connection.",
        );
        ui.add_space(14.0);
        labeled_text_field(
            ui,
            "Profile name",
            &mut app.profile_name,
            "Production cluster",
        );
        labeled_text_field(
            ui,
            "Server URLs",
            &mut app.server_url,
            "nats://host:4222 (comma-separated for clusters)",
        );

        ui.add_space(5.0);
        ui.checkbox(
            &mut app.use_password_auth,
            "Use username and password authentication",
        );
        if app.use_password_auth {
            ui.horizontal(|ui| {
                labeled_text_field(ui, "Username", &mut app.username, "NATS username");
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Password")
                            .small()
                            .strong()
                            .color(theme::MUTED),
                    );
                    ui.add_sized(
                        [240.0, 40.0],
                        egui::TextEdit::singleline(&mut app.password)
                            .password(true)
                            .hint_text("Stored in system keychain"),
                    );
                });
            });
        }

        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if primary_button(ui, "Save profile securely").clicked() {
                app.save_profile();
            }
            if ui.button("Import .creds").clicked() {
                app.import_credentials_file();
            }
            if ui.button("Import NATS CLI context").clicked() {
                app.import_nats_context();
            }
        });

        ui.add_space(16.0);
        separator_label(ui, "TLS / MTLS MATERIAL");
        ui.label(
            RichText::new("PEM files are stored in the operating system credential store.")
                .small()
                .color(theme::MUTED),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.button("Import CA certificate").clicked() {
                app.import_tls_material(TlsMaterial::CaCertificate);
            }
            if ui.button("Import client certificate").clicked() {
                app.import_tls_material(TlsMaterial::ClientCertificate);
            }
            if ui.button("Import private key").clicked() {
                app.import_tls_material(TlsMaterial::ClientPrivateKey);
            }
        });
        ui.small(format!(
            "CA {}   ·   Certificate {}   ·   Private key {}",
            imported_state(app.tls_ca_certificate_key.is_some()),
            imported_state(app.tls_client_certificate_key.is_some()),
            imported_state(app.tls_client_private_key_key.is_some()),
        ));
        feedback_line(ui, &app.status);
    });
}

fn show_overview(app: &mut NatsManagerApp, ui: &mut egui::Ui) {
    if app.active_client.is_none() {
        empty_state(
            ui,
            "No cluster connected",
            "Connect to an existing NATS server to see live health and resource usage.",
            "Open connections",
            || app.current_page = AppPage::Connections,
        );
        return;
    }

    card(ui, |ui| {
        ui.horizontal(|ui| {
            status_dot(ui, theme::SUCCESS);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Connection healthy")
                        .size(18.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(RichText::new(&app.status).color(theme::MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(info) = &app.server_info {
                    badge(
                        ui,
                        format!("NATS {}", info.version),
                        theme::ACCENT,
                        theme::ACCENT_DARK,
                    );
                }
            });
        });
    });

    if let Some(info) = &app.server_info {
        ui.columns(4, |columns| {
            metric_card(
                &mut columns[0],
                "SERVER",
                &info.server_name,
                &info.server_id,
                theme::ACCENT,
            );
            metric_card(
                &mut columns[1],
                "ADDRESS",
                &format!("{}:{}", info.host, info.port),
                "Connected endpoint",
                theme::SUCCESS,
            );
            match &app.jetstream_status {
                Some(JetStreamStatus::Enabled { streams, .. }) => {
                    metric_card(
                        &mut columns[2],
                        "STREAMS",
                        &streams.to_string(),
                        "JetStream enabled",
                        theme::ACCENT,
                    );
                }
                Some(JetStreamStatus::Unavailable(_)) => {
                    metric_card(
                        &mut columns[2],
                        "JETSTREAM",
                        "Unavailable",
                        "Core NATS remains available",
                        theme::WARNING,
                    );
                }
                None => metric_card(
                    &mut columns[2],
                    "JETSTREAM",
                    "Checking…",
                    "Capability discovery",
                    theme::MUTED,
                ),
            }
            if let Some(JetStreamStatus::Enabled { consumers, .. }) = &app.jetstream_status {
                metric_card(
                    &mut columns[3],
                    "CONSUMERS",
                    &consumers.to_string(),
                    "Across this account",
                    theme::SUCCESS,
                );
            } else {
                metric_card(
                    &mut columns[3],
                    "CLIENT IP",
                    &info.client_ip.to_string(),
                    "This connection",
                    theme::MUTED,
                );
            }
        });

        card(ui, |ui| {
            section_heading(
                ui,
                "Server details",
                "Identity and topology advertised by the active NATS server.",
            );
            ui.add_space(12.0);
            detail_row(ui, "Server name", &info.server_name);
            detail_row(ui, "Server ID", &info.server_id);
            detail_row(ui, "Address", &format!("{}:{}", info.host, info.port));
            detail_row(
                ui,
                "Cluster",
                info.cluster.as_deref().unwrap_or("Not advertised"),
            );
            detail_row(
                ui,
                "Domain",
                info.domain.as_deref().unwrap_or("Not advertised"),
            );
            detail_row(ui, "Max payload", &format_bytes(info.max_payload as u64));
            detail_row(
                ui,
                "Advertised servers",
                if info.connect_urls.is_empty() {
                    "No additional servers advertised".to_owned()
                } else {
                    info.connect_urls.join(", ")
                }
                .as_str(),
            );
        });
    }

    if let Some(status) = &app.jetstream_status {
        match status {
            JetStreamStatus::Enabled {
                streams,
                consumers,
                memory_bytes,
                storage_bytes,
                domain,
            } => {
                card(ui, |ui| {
                    section_heading(
                        ui,
                        "JetStream account",
                        "Current resource usage reported by this account.",
                    );
                    ui.add_space(12.0);
                    ui.columns(3, |columns| {
                        metric_card(
                            &mut columns[0],
                            "STREAMS",
                            &streams.to_string(),
                            "Configured streams",
                            theme::ACCENT,
                        );
                        metric_card(
                            &mut columns[1],
                            "CONSUMERS",
                            &consumers.to_string(),
                            "Configured consumers",
                            theme::SUCCESS,
                        );
                        metric_card(
                            &mut columns[2],
                            "STORAGE",
                            &format_bytes(*storage_bytes),
                            &format!(
                                "Memory {} · Domain {}",
                                format_bytes(*memory_bytes),
                                domain.as_deref().unwrap_or("default")
                            ),
                            theme::WARNING,
                        );
                    });
                });
            }
            JetStreamStatus::Unavailable(reason) => {
                card(ui, |ui| {
                    section_heading(
                        ui,
                        "JetStream not enabled",
                        "Core NATS messaging is still available.",
                    );
                    ui.label(RichText::new(reason).color(theme::MUTED));
                });
            }
        }
    }
}

fn show_jetstream(app: &mut NatsManagerApp, ui: &mut egui::Ui) {
    match (&app.active_client, &app.jetstream_status) {
        (None, _) => {
            empty_state(
                ui,
                "Connect to a cluster first",
                "JetStream resources appear after connecting to a server with JetStream enabled.",
                "Open connections",
                || app.current_page = AppPage::Connections,
            );
            return;
        }
        (_, Some(JetStreamStatus::Unavailable(reason))) => {
            card(ui, |ui| {
                badge(
                    ui,
                    "JETSTREAM UNAVAILABLE",
                    theme::WARNING,
                    theme::DANGER_DARK,
                );
                ui.add_space(12.0);
                ui.label(RichText::new(reason).color(theme::MUTED));
                ui.label(
                    RichText::new("Use the Publish screen for core NATS messages.")
                        .small()
                        .color(theme::MUTED),
                );
            });
            return;
        }
        (_, None) => {
            empty_state(
                ui,
                "Checking JetStream",
                "Waiting for the connected server to report account capabilities.",
                "",
                || {},
            );
            return;
        }
        (_, Some(JetStreamStatus::Enabled { .. })) => {}
    }

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            section_heading(
                ui,
                "Stream explorer",
                "Select a stream to inspect its subjects, consumers, and retained messages.",
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("↻  Refresh").clicked() {
                app.refresh_streams();
            }
        });
    });
    ui.add_space(12.0);

    ui.columns(2, |columns| {
        card(&mut columns[0], |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Streams")
                            .size(17.0)
                            .strong()
                            .color(theme::TEXT),
                    );
                    ui.label(
                        RichText::new(format!("{} resources", app.streams.len()))
                            .small()
                            .color(theme::MUTED),
                    );
                });
            });
            ui.add_space(10.0);
            if app.streams.is_empty() {
                inline_empty_state(
                    ui,
                    "No streams found",
                    "Create the first stream using the form below.",
                );
            }
            for stream in app.streams.clone() {
                let selected = app.selected_stream.as_ref() == Some(&stream);
                let response = ui.add_sized(
                    [ui.available_width(), 42.0],
                    egui::Button::new(RichText::new(format!("▤   {stream}")).color(if selected {
                        theme::ACCENT
                    } else {
                        theme::TEXT
                    }))
                    .fill(if selected {
                        theme::ACCENT_DARK
                    } else {
                        theme::SURFACE_RAISED
                    })
                    .stroke(Stroke::new(
                        1.0,
                        if selected {
                            theme::ACCENT.gamma_multiply(0.4)
                        } else {
                            theme::SURFACE_RAISED
                        },
                    ))
                    .corner_radius(8),
                );
                if response.clicked() {
                    app.selected_stream = Some(stream);
                    app.selected_consumer = None;
                    app.consumers.clear();
                    app.stream_details = None;
                    app.refresh_stream_details();
                    app.refresh_consumers();
                }
                ui.add_space(5.0);
            }
        });

        card(&mut columns[1], |ui| {
            if let Some(stream) = app.selected_stream.clone() {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&stream)
                                .size(18.0)
                                .strong()
                                .color(theme::TEXT),
                        );
                        ui.label(
                            RichText::new("STREAM CONFIGURATION")
                                .small()
                                .color(theme::ACCENT),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Inspect messages").clicked() {
                            app.inspect_recent_messages();
                        }
                    });
                });
                ui.add_space(14.0);
                if let Some(details) = &app.stream_details {
                    ui.columns(3, |columns| {
                        compact_metric(&mut columns[0], "MESSAGES", &details.messages.to_string());
                        compact_metric(&mut columns[1], "STORAGE", &format_bytes(details.bytes));
                        compact_metric(
                            &mut columns[2],
                            "CONSUMERS",
                            &details.consumers.to_string(),
                        );
                    });
                    detail_row(ui, "Subjects", &details.subjects.join(", "));
                } else {
                    ui.label(RichText::new("Loading stream details…").color(theme::MUTED));
                }

                ui.add_space(10.0);
                separator_label(ui, "UPDATE SUBJECT FILTER");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() * 0.72, 40.0],
                        egui::TextEdit::singleline(&mut app.new_stream_subject)
                            .hint_text("orders.>"),
                    );
                    if primary_button(ui, "Update").clicked() {
                        app.update_selected_stream_subject();
                    }
                });

                ui.add_space(16.0);
                separator_label(ui, "CONSUMERS");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() * 0.67, 40.0],
                        egui::TextEdit::singleline(&mut app.new_consumer_name)
                            .hint_text("New durable pull consumer"),
                    );
                    if primary_button(ui, "Create").clicked() {
                        app.create_consumer();
                    }
                });
                for consumer in app.consumers.clone() {
                    let selected = app.selected_consumer.as_ref() == Some(&consumer);
                    if ui
                        .selectable_label(
                            selected,
                            RichText::new(format!("◉  {consumer}")).color(if selected {
                                theme::ACCENT
                            } else {
                                theme::TEXT
                            }),
                        )
                        .clicked()
                    {
                        app.selected_consumer = Some(consumer);
                        app.refresh_consumer_details();
                    }
                }
                if let Some(consumer) = app.selected_consumer.clone() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Consumer limits · {consumer}"))
                            .strong()
                            .color(theme::TEXT),
                    );
                    ui.horizontal(|ui| {
                        egui::Grid::new("consumer_limits_grid")
                            .num_columns(4)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Max deliveries").small().color(theme::MUTED),
                                );
                                ui.add_sized(
                                    [80.0, 36.0],
                                    egui::TextEdit::singleline(&mut app.max_deliver_input),
                                );
                                ui.label(RichText::new("Ack wait (s)").small().color(theme::MUTED));
                                ui.add_sized(
                                    [80.0, 36.0],
                                    egui::TextEdit::singleline(&mut app.ack_wait_input),
                                );
                                ui.end_row();
                            });
                        if primary_button(ui, "Save limits").clicked() {
                            app.update_selected_consumer_limits();
                        }
                    });
                }
            } else {
                inline_empty_state(
                    ui,
                    "Select a stream",
                    "Stream details and consumer controls will appear here.",
                );
            }
            feedback_line(ui, &app.jetstream_message);
        });
    });

    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_heading(
                    ui,
                    "Recent messages",
                    "Inspect, replay, and verify retained message data.",
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.selected_message.is_some() && ui.button("Replay selected").clicked() {
                    app.replay_selected_message();
                }
            });
        });
        if app.messages.is_empty() {
            inline_empty_state(
                ui,
                "No messages loaded",
                "Select a stream and choose Inspect messages.",
            );
        } else {
            for message in app.messages.clone() {
                let preview: String = String::from_utf8_lossy(&message.payload)
                    .chars()
                    .take(120)
                    .collect();
                let label = format!(
                    "#{:>6}   {}   {}",
                    message.sequence, message.subject, preview
                );
                ui.selectable_value(&mut app.selected_message, Some(message.sequence), label);
            }
        }
        feedback_line(ui, &app.jetstream_message);
    });

    card(ui, |ui| {
        section_heading(
            ui,
            "Create stream",
            "Define a stream and the subject filter it owns.",
        );
        ui.horizontal(|ui| {
            ui.add_sized(
                [ui.available_width() * 0.38, 40.0],
                egui::TextEdit::singleline(&mut app.new_stream_name).hint_text("ORDERS"),
            );
            ui.add_sized(
                [ui.available_width() * 0.58, 40.0],
                egui::TextEdit::singleline(&mut app.new_stream_subject).hint_text("orders.>"),
            );
            if primary_button(ui, "Create stream").clicked() {
                app.create_stream();
            }
        });
    });
}

fn show_publish(app: &mut NatsManagerApp, ui: &mut egui::Ui) {
    if app.active_client.is_none() {
        empty_state(
            ui,
            "Connect before publishing",
            "Choose a saved profile or make a quick connection first.",
            "Open connections",
            || app.current_page = AppPage::Connections,
        );
        return;
    }

    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Message composer")
                        .size(18.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new(
                        "Payloads are sent exactly as entered. Choose the publishing path below.",
                    )
                    .color(theme::MUTED),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.publish_via_jetstream {
                    badge(ui, "JETSTREAM ACK", theme::ACCENT, theme::ACCENT_DARK);
                } else {
                    badge(ui, "CORE NATS", theme::SUCCESS, theme::ACCENT_DARK);
                }
            });
        });
        ui.add_space(18.0);
        ui.label(
            RichText::new("SUBJECT")
                .small()
                .strong()
                .color(theme::MUTED),
        );
        ui.add_sized(
            [ui.available_width(), 42.0],
            egui::TextEdit::singleline(&mut app.publish_subject).hint_text("orders.created"),
        );
        ui.add_space(12.0);
        ui.label(
            RichText::new("PAYLOAD")
                .small()
                .strong()
                .color(theme::MUTED),
        );
        ui.add_sized(
            [ui.available_width(), 230.0],
            egui::TextEdit::multiline(&mut app.publish_payload)
                .hint_text("Write message payload…")
                .code_editor(),
        );
        ui.add_space(10.0);
        if matches!(&app.jetstream_status, Some(JetStreamStatus::Enabled { .. })) {
            ui.checkbox(
                &mut app.publish_via_jetstream,
                "Publish through JetStream and wait for the storage acknowledgement",
            );
        } else {
            ui.label(
                RichText::new("This cluster has no JetStream; publishing uses core NATS.")
                    .small()
                    .color(theme::MUTED),
            );
        }
        ui.add_space(9.0);
        ui.horizontal(|ui| {
            if primary_button(ui, "Publish message").clicked() {
                app.publish_message();
            }
            feedback_line(ui, &app.jetstream_message);
        });
    });
}

fn show_maintenance(app: &mut NatsManagerApp, ui: &mut egui::Ui) {
    if app.active_client.is_none() {
        empty_state(
            ui,
            "No active cluster",
            "Connect first, then select resources from the JetStream screen.",
            "Open connections",
            || app.current_page = AppPage::Connections,
        );
        return;
    }
    if let Some(JetStreamStatus::Unavailable(reason)) = &app.jetstream_status {
        card(ui, |ui| {
            badge(
                ui,
                "JETSTREAM UNAVAILABLE",
                theme::WARNING,
                theme::DANGER_DARK,
            );
            ui.add_space(12.0);
            ui.label(RichText::new(reason).color(theme::MUTED));
        });
        return;
    }
    if !matches!(&app.jetstream_status, Some(JetStreamStatus::Enabled { .. })) {
        inline_empty_state(
            ui,
            "JetStream capability not confirmed",
            "Wait for cluster discovery to finish.",
        );
        return;
    }

    card(ui, |ui| {
        ui.horizontal(|ui| {
            status_dot(ui, theme::DANGER);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Destructive operations")
                        .size(18.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new(
                        "This screen is intentionally separate from routine resource management.",
                    )
                    .color(theme::MUTED),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                badge(
                    ui,
                    "EXACT-NAME CONFIRMATION",
                    theme::WARNING,
                    theme::DANGER_DARK,
                );
            });
        });
    });

    card(ui, |ui| {
        section_heading(
            ui,
            "Stream maintenance",
            "Select a stream in JetStream before deleting it or purging its messages.",
        );
        ui.add_space(10.0);
        if let Some(stream) = app.selected_stream.clone() {
            detail_row(ui, "Selected stream", &stream);
            ui.add_space(8.0);
            labeled_text_field(
                ui,
                "Type the exact stream name",
                &mut app.stream_name_confirmation,
                &stream,
            );
            ui.horizontal_wrapped(|ui| {
                let confirmed = nats_manager::safety::confirms_resource_name(
                    &stream,
                    &app.stream_name_confirmation,
                );
                if danger_button(ui, "Delete stream permanently", !confirmed).clicked() {
                    app.delete_selected_stream();
                }
                if danger_button(ui, "Purge all messages", !confirmed).clicked() {
                    app.purge_selected_stream();
                }
            });
        } else {
            inline_empty_state(
                ui,
                "No stream selected",
                "Go to JetStream and select a stream to make it available here.",
            );
        }
    });

    card(ui, |ui| {
        section_heading(
            ui,
            "Consumer maintenance",
            "Deleting a consumer does not delete its stream.",
        );
        ui.add_space(10.0);
        if let Some(consumer) = app.selected_consumer.clone() {
            detail_row(ui, "Selected consumer", &consumer);
            ui.add_space(8.0);
            labeled_text_field(
                ui,
                "Type the exact consumer name",
                &mut app.consumer_name_confirmation,
                &consumer,
            );
            let confirmed = nats_manager::safety::confirms_resource_name(
                &consumer,
                &app.consumer_name_confirmation,
            );
            if danger_button(ui, "Delete consumer permanently", !confirmed).clicked() {
                app.delete_selected_consumer();
            }
        } else {
            inline_empty_state(
                ui,
                "No consumer selected",
                "Go to JetStream and select a consumer to make it available here.",
            );
        }
        feedback_line(ui, &app.jetstream_message);
    });
}

fn profile_row(app: &mut NatsManagerApp, ui: &mut egui::Ui, profile: ConnectionProfile) {
    let authentication_label = auth_label(&profile.authentication);
    egui::Frame::new()
        .fill(theme::SURFACE_RAISED)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(9)
        .inner_margin(egui::Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&profile.name).strong().color(theme::TEXT));
                    ui.label(
                        RichText::new(profile.servers.join("  ·  "))
                            .small()
                            .color(theme::MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Connect").clicked() {
                        app.connect_saved_profile(profile);
                    }
                    badge(ui, authentication_label, theme::ACCENT, theme::ACCENT_DARK);
                });
            });
        });
    ui.add_space(7.0);
}

fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(13)
        .inner_margin(egui::Margin::same(18))
        .show(ui, add_contents);
    ui.add_space(13.0);
}

fn section_heading(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(RichText::new(title).size(17.0).strong().color(theme::TEXT));
    ui.label(RichText::new(description).small().color(theme::MUTED));
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: &str, note: &str, tint: Color32) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            status_dot(ui, tint);
            ui.label(RichText::new(label).small().strong().color(theme::MUTED));
        });
        ui.add_space(8.0);
        ui.label(RichText::new(value).size(19.0).strong().color(theme::TEXT));
        ui.label(RichText::new(note).small().color(theme::MUTED));
    });
}

fn compact_metric(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().strong().color(theme::MUTED));
        ui.label(RichText::new(value).strong().color(theme::TEXT));
    });
}

fn detail_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [142.0, 23.0],
            egui::Label::new(RichText::new(label).small().color(theme::MUTED)),
        );
        ui.label(RichText::new(value).color(theme::TEXT));
    });
}

fn labeled_text_field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [142.0, 23.0],
            egui::Label::new(RichText::new(label).small().strong().color(theme::MUTED)),
        );
        ui.add_sized(
            [ui.available_width(), 40.0],
            egui::TextEdit::singleline(value).hint_text(hint),
        );
    });
}

fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .strong()
                .color(Color32::from_rgb(8, 25, 32)),
        )
        .fill(theme::ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(9),
    )
}

fn danger_button(ui: &mut egui::Ui, label: &str, disabled: bool) -> egui::Response {
    ui.add_enabled(
        !disabled,
        egui::Button::new(RichText::new(label).strong().color(theme::DANGER))
            .fill(theme::DANGER_DARK)
            .stroke(Stroke::new(1.0, theme::DANGER.gamma_multiply(0.45)))
            .corner_radius(9),
    )
}

fn badge(ui: &mut egui::Ui, label: impl Into<String>, tint: Color32, fill: Color32) {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, tint.gamma_multiply(0.25)))
        .corner_radius(20)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(label.into()).small().strong().color(tint));
        });
}

fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (response, painter) = ui.allocate_painter(vec2(8.0, 8.0), egui::Sense::hover());
    painter.circle_filled(response.rect.center(), 4.0, color);
}

fn feedback_line(ui: &mut egui::Ui, message: &str) {
    if !message.is_empty() {
        ui.add_space(8.0);
        let color = if message.to_ascii_lowercase().contains("fail")
            || message.to_ascii_lowercase().contains("could not")
        {
            theme::DANGER
        } else {
            theme::MUTED
        };
        ui.label(RichText::new(message).small().color(color));
    }
}

fn separator_label(ui: &mut egui::Ui, label: &str) {
    ui.add_space(4.0);
    ui.separator();
    ui.label(RichText::new(label).small().strong().color(theme::MUTED));
    ui.add_space(5.0);
}

fn inline_empty_state(ui: &mut egui::Ui, title: &str, description: &str) {
    egui::Frame::new()
        .fill(theme::CANVAS)
        .corner_radius(9)
        .inner_margin(egui::Margin::same(15))
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().color(theme::TEXT));
            ui.label(RichText::new(description).small().color(theme::MUTED));
        });
}

fn empty_state(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    action: &str,
    on_action: impl FnOnce(),
) {
    card(ui, |ui| {
        ui.vertical_centered(|ui| {
            draw_brand_mark(ui, 58.0);
            ui.add_space(12.0);
            ui.heading(title);
            ui.label(RichText::new(description).color(theme::MUTED));
            if !action.is_empty() {
                ui.add_space(10.0);
                if primary_button(ui, action).clicked() {
                    on_action();
                }
            }
        });
    });
}

fn auth_label(authentication: &Authentication) -> &'static str {
    match authentication {
        Authentication::None => "NO AUTH",
        Authentication::UserPassword { .. } => "USER / PASSWORD",
        Authentication::Token { .. } => "TOKEN",
        Authentication::NKey { .. } => "NKEY",
        Authentication::Jwt { .. } => "JWT",
        Authentication::CredentialsFile { .. } => "CREDS",
    }
}

fn imported_state(imported: bool) -> &'static str {
    if imported { "imported" } else { "not set" }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
