use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("zero2prod_test", "debug", std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("zero2prod_test", "debug", std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub fn get_confirmation_link(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);

            let raw_link = links[0].as_str().to_owned();

            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");

            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        ConfirmationLinks {
            html: get_link(body["HtmlBody"].as_str().unwrap()),
            text: get_link(body["TextBody"].as_str().unwrap()),
        }
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration = {
        let mut configuration = get_configuration().expect("Failed to read configuration");
        configuration.application.port = 0;
        configuration.database.name = Uuid::new_v4().to_string();
        configuration.email_client.base_url = email_server.uri();
        configuration
    };

    let db_pool = configure_database(&configuration.database).await;

    let application = Application::build(configuration);
    let address = format!("http://{}", application.address());
    let port = application.port();

    let _ = tokio::spawn(async move { application.run().await.expect("Failed to run the server") });

    TestApp {
        address,
        port,
        db_pool,
        email_server,
    }
}

async fn configure_database(settings: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&settings.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, settings.name).as_str())
        .await
        .expect("Failed to create database");

    let db_pool = get_connection_pool(&settings);

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    db_pool
}
