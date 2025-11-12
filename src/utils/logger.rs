use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub fn init_logging(log_config: &crate::config::LogConfig) {
    // Create file appender for JSON logs if enabled and log_dir is provided
    let file_layer = if log_config.file_enabled {
        log_config.dir.as_ref().map(|dir| {
            let file_appender = tracing_appender::rolling::daily(dir, "app.log");
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

            // Leak the guard to keep it alive for the lifetime of the program
            std::mem::forget(_guard);

            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::CLOSE)
                .with_current_span(true)
                .with_thread_ids(true)
                .with_target(true)
                .boxed()
        })
    } else {
        None
    };

    // Console layer with configurable formatting
    let console_layer = match log_config.console_format.as_str() {
        "json" => fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(false)
            .boxed(),
        "pretty" | _ => fmt::layer()
            .pretty()
            .with_target(true)
            .with_thread_ids(false)
            .with_file(true)
            .with_line_number(true)
            .boxed(),
    };

    // Build the filter with base level and ext_level overrides
    let mut filter_string = log_config.level.clone();

    // Add ext_level directives if configured
    if let Some(ext_levels) = &log_config.ext_level {
        for (target, level) in ext_levels {
            filter_string.push_str(&format!(",{}={}", target, level));
        }
    }

    // Create filter from constructed string or environment variable
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&filter_string));

    // Build the subscriber
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(console_layer);

    // Add file layer if configured
    if let Some(file_layer) = file_layer {
        subscriber.with(file_layer).init();
    } else {
        subscriber.init();
    }
}
