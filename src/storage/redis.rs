pub mod redis {
    use crate::storage::storage::TagStorage;
    use config::Config;
    use redis::{Client, Commands};

    pub struct RedisTagStorage {
        client: Client,
    }

    impl RedisTagStorage {
        pub fn new(config: &Config) -> Self {
            let client_url = format!(
                "redis://{}:{}/",
                config.get::<String>("storage-redis.host").unwrap(),
                config.get::<String>("storage-redis.port").unwrap()
            );
            let client = Client::open(client_url).unwrap();
            Self { client }
        }
    }

    impl TagStorage for RedisTagStorage {
        fn add_tags(&self, user_id: String, text: String, tags: Vec<String>) -> anyhow::Result<()> {
            for tag in tags {
                let key: String = format!("classify:{}:{}", user_id, tag.trim().to_lowercase());
                let mut conn = self.client.get_connection().unwrap();
                let _: () = conn.sadd(key, &text)?;
            }
            Ok(())
        }

        fn get_tags(&self, user_id: String) -> anyhow::Result<Vec<String>> {
            let mut conn = self.client.get_connection().unwrap();
            let keys: Vec<String> = conn.keys(format!("classify:{}:*", user_id))?;
            let tags: Vec<String> = keys.iter().map(|key| key.split(":").nth(2).unwrap().to_string()).collect();
            Ok(tags)
        }

        fn get_texts(&self, user_id: String, tag: String) -> anyhow::Result<Vec<String>> {
            let key: String = format!("classify:{}:{}", user_id, tag);
            let mut conn = self.client.get_connection().unwrap();
            let texts: Vec<String> = conn.smembers(key)?;
            Ok(texts)
        }
    }
}
