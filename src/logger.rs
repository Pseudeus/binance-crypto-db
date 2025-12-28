use tracing_subscriber::EnvFilter;

pub fn setup_logger() {
    let filter = EnvFilter::new("debug").add_directive("sqlx=warn".parse().unwrap());

    tracing_subscriber::fmt()
        // .with_file(true)
        // .with_line_number(true)
        .with_target(true)
        // .with_thread_ids(true)
        .with_level(true)
        .with_ansi(true)
        .compact()
        .with_env_filter(filter)
        .init();
}
