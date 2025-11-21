use leaf::{
    cli::{Cli, Context},
    config::Settings,
    services::AppServices,
    utils,
};

#[tokio::main]
async fn main() {
    // Initialize basic logging FIRST, before anything can fail
    let settings = match Settings::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            eprintln!("Error details: {:?}", e);
            std::process::exit(1);
        }
    };

    utils::logger::init_logging(&settings.logs);

    let cli = Cli::parse_args();

    if !cli.should_run_main() {
        let app_services = match AppServices::new(&settings).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to initialize services: {}", e);
                std::process::exit(1);
            }
        };

        cli.execute(&Context {
            settings: &settings,
            services: &app_services,
        })
        .await;
        return;
    }
}
