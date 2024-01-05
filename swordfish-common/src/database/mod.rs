pub mod katana;

use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use std::env;
use std::sync::OnceLock;
use tracing::info;

static MONGO_CLIENT: OnceLock<Client> = OnceLock::new();
static MONGO_DATABASE: OnceLock<Database> = OnceLock::new();

async fn init() {
    let mut options =
        ClientOptions::parse(env::var("MONGODB_URL").expect("MongoDB url must be provided"))
            .await
            .unwrap();
    match env::var("MONGODB_USERNAME") {
        Ok(username) => {
            options.credential = Some(
                mongodb::options::Credential::builder()
                    .username(username)
                    .password(
                        env::var("MONGODB_PASSWORD").expect("MongoDB password must be provided"),
                    )
                    .build(),
            );
        }
        Err(_) => {
            info!("No MongoDB username provided, using authentication provided in the url");
        }
    }
    MONGO_CLIENT
        .set(Client::with_options(options).unwrap())
        .unwrap();
    MONGO_DATABASE.set(MONGO_CLIENT.get().unwrap().database("swordfish"));
    katana::init();
}
