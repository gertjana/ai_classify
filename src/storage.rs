#[path = "storage/redis.rs"]
mod redis;

pub mod storage {
    use super::redis::redis::RedisTagStorage;
    use config::Config;

    pub trait TagStorage {
        fn add_tags(&self, user_id: String, text: String, tags: Vec<String>) -> anyhow::Result<()>;
        fn get_tags(&self, user_id: String) -> anyhow::Result<Vec<String>>;
        fn get_texts(&self, user_id: String, tag: String) -> anyhow::Result<Vec<String>>;
    }

    // pub trait BlobStorage {
    //     fn add_blob(&self, user_id: String, blob: String) -> anyhow::Result<()>;
    //     fn get_blob(&self, user_id: String) -> anyhow::Result<String>;
    // }

    pub fn store_tags(
        user_id: String,
        text: String,
        tags: Vec<String>,
        config: &Config,
    ) -> anyhow::Result<()> {
        let storage_type = config.get::<String>("general.tag-storage-type").unwrap();
        let storage: Box<dyn TagStorage> = match storage_type.as_str() {
            "redis" => Box::new(RedisTagStorage::new(config)),
            _ => panic!("Unsupported storage type: {}", storage_type),
        };
        storage.add_tags(user_id, text, tags)
    }

    pub fn get_tags(user_id: String, config: &Config) -> anyhow::Result<Vec<String>> {
        let storage_type = config.get::<String>("general.tag-storage-type").unwrap();
        let storage: Box<dyn TagStorage> = match storage_type.as_str() {
            "redis" => Box::new(RedisTagStorage::new(config)),
            _ => panic!("Unsupported storage type: {}", storage_type),
        };
        storage.get_tags(user_id)
    }

    pub fn get_texts(user_id: String, tag: String, config: &Config) -> anyhow::Result<Vec<String>> {
        let storage_type = config.get::<String>("general.tag-storage-type").unwrap();
        let storage: Box<dyn TagStorage> = match storage_type.as_str() {
            "redis" => Box::new(RedisTagStorage::new(config)),
            _ => panic!("Unsupported storage type: {}", storage_type),
        };
        storage.get_texts(user_id, tag)
    }
}
