pub mod katana;

use mongodb::bson::doc;
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use std::env;
use tokio::sync::OnceCell;
use tracing::info;

static MONGO_CLIENT: OnceCell<Client> = OnceCell::const_new();
static MONGO_DATABASE: OnceCell<Database> = OnceCell::const_new();

pub async fn init() {
    let mut options =
        ClientOptions::parse(env::var("MONGODB_URL").expect("MongoDB url must be provided"))
            .await
            .unwrap();
    options.direct_connection = Some(true);
    options.app_name = Some("swordfish".to_string());
    options.default_database = Some("swordfish".to_string());
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
    let client = Client::with_options(options).unwrap();
    let db = client.database("swordfish");
    db.run_command(doc! { "ping": 1 }, None)
        .await
        .expect("Failed to connect to MongoDB");
    MONGO_DATABASE.set(db).unwrap();
    MONGO_CLIENT.set(client).unwrap();
    katana::init();
}
