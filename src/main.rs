use sqlx::postgres::PgPoolOptions;
use structopt::StructOpt;
use zero2prod::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[derive(StructOpt)]
struct Migrate {
    /// It will retry this number of times before giving up
    #[structopt(long, default_value = "0")]
    retry: u64,

    /// Make migration sleep this amount of time before each retry
    #[structopt(long, default_value = "0")]
    retry_delay: u64,

    /// Maximum time in seconds that you allow connection to take
    #[structopt(long, default_value = "2")]
    timeout: u64,
}

#[derive(StructOpt)]
enum Opt {
    /// Execute database migration
    Migrate(Migrate),
    /// Run the service
    Serve,
}

async fn migrate(opt: Migrate) {
    let configuration = get_configuration().expect("Failed to read configuration");

    for retry in 0..=opt.retry {
        if retry > 0 {
            println!("Retry number {} (waiting {}s)", retry, opt.retry_delay);
            std::thread::sleep(std::time::Duration::from_secs(opt.retry_delay));
        }

        match PgPoolOptions::new()
            .connect_timeout(std::time::Duration::from_secs(opt.timeout))
            .connect_with(configuration.database.with_db())
            .await
        {
            Ok(pool) => {
                sqlx::migrate!("./migrations")
                    .run(&pool)
                    .await
                    .expect("Failed to migrate the database");

                println!("Migration completed with success");
                std::process::exit(0);
            }
            Err(e) => {
                println!("Failed to connect: {}", e);
            }
        }
    }
    std::process::exit(1);
}

async fn run() -> hyper::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration");

    Application::build(configuration).run().await
}

#[tokio::main]
async fn main() -> hyper::Result<()> {
    let subscriber = get_subscriber("zero2prod", "info", std::io::stdout);
    init_subscriber(subscriber);

    match Opt::from_args() {
        Opt::Migrate(opt) => migrate(opt).await,
        Opt::Serve => run().await?,
    }
    Ok(())
}
