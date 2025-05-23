// use crate::storage::ContentStorage;
// use crate::{ClassifyResult, Content};
// use std::env;

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::storage::content::s3::S3ContentStorage;

    // #[tokio::test]
    // #[ignore]
    // async fn test_s3_storage_integration() -> ClassifyResult<()> {
    //     // This test requires valid AWS credentials and a test bucket
    //     // It's marked as 'ignore' so it doesn't run in normal test runs

    //     let bucket = env::var("TEST_S3_BUCKET").expect("TEST_S3_BUCKET must be set for S3 tests");
    //     let region = env::var("TEST_S3_REGION").expect("TEST_S3_REGION must be set for S3 tests");
    //     let prefix = format!("test-{}/", uuid::Uuid::new_v4());

    //     let storage = S3ContentStorage::new(
    //         &bucket, &prefix, &region, None, // use default profile
    //         None, // no explicit access key
    //         None, // no explicit secret key
    //     )
    //     .await?;

    //     let content = Content::new("S3 storage test content".to_string())
    //         .with_tags(vec!["test".to_string(), "s3".to_string()]);
    //     let content_id = content.id.to_string();

    //     storage.store(&content).await?;

    //     let retrieved = storage.get(&content_id).await?;
    //     assert!(retrieved.is_some());
    //     let retrieved = retrieved.unwrap();
    //     assert_eq!(retrieved.id, content.id);
    //     assert_eq!(retrieved.content, content.content);
    //     assert_eq!(retrieved.tags, content.tags);

    //     let contents = storage.list().await?;
    //     assert_eq!(contents.len(), 1);

    //     let hash = content.content_hash.as_ref().unwrap();
    //     let found = storage.find_by_hash(hash).await?;
    //     assert!(found.is_some());
    //     assert_eq!(found.unwrap().id, content.id);

    //     let deleted = storage.delete(&content_id).await?;
    //     assert!(deleted);

    //     let retrieved = storage.get(&content_id).await?;
    //     assert!(retrieved.is_none());

    //     let deleted = storage.delete(&content_id).await?;
    //     assert!(!deleted);

    //     Ok(())
    // }
}
