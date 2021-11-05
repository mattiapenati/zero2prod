use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;
use structopt::StructOpt;
use zero2prod::{
    configuration::get_configuration,
    startup::run,
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

#[tokio::main]
async fn main() {
    let subscriber = get_subscriber("zero2prod", "info", std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration");

    match Opt::from_args() {
        Opt::Migrate(opt) => {
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
        Opt::Serve => {
            let connection_pool = PgPoolOptions::new()
                .connect_timeout(std::time::Duration::from_secs(2))
                .connect_lazy_with(configuration.database.with_db());

            let listener = TcpListener::bind(&configuration.application.address()).unwrap();

            run(listener, connection_pool).await.unwrap();
        }
    }
}
