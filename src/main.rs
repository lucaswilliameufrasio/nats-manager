mod app;

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create Tokio runtime");

    eframe::run_native(
        "NATS Manager",
        eframe::NativeOptions::default(),
        Box::new(move |creation_context| {
            app::configure_theme(&creation_context.egui_ctx);
            Ok(Box::new(app::NatsManagerApp::new(runtime)))
        }),
    )
}
