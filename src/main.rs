use leaf::{
    cli::{Cli, Context},
    config::Settings,
    services::AppServices,
    utils,
};

#[tokio::main]
async fn main() {
    let settings = Settings::new().expect("Failed to load configuration");
    let cli = Cli::parse_args();

    utils::logger::init_logging(&settings.logs);

    if !cli.should_run_main() {
        // Execute CLI command and exit
        let app_services = AppServices::new(&settings)
            .await
            .expect("Failed to initialize services");

        cli.execute(&Context {
            settings: &settings,
            services: &app_services,
        })
        .await;
        return;
    }
}
