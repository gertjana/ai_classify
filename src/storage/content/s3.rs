use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{config::Region, Client as S3Client};
use tokio::io::AsyncReadExt;

use crate::storage::ContentStorage;
use crate::{ClassifyError, ClassifyResult, Content};

/// S3-based content storage
pub struct S3ContentStorage {
    client: S3Client,
    bucket: String,
    prefix: String,
}

impl S3ContentStorage {
    pub async fn new(
        bucket: &str,
        prefix: &str,
        region: &str,
        profile: Option<&str>,
        access_key: Option<&str>,
        secret_key: Option<&str>,
    ) -> ClassifyResult<Self> {
        let region = Region::new(region.to_string());

        let mut builder = aws_config::from_env().region(region);

        if let Some(profile) = profile {
            builder = builder.profile_name(profile);
        } else if let (Some(access_key), Some(secret_key)) = (access_key, secret_key) {
            let credentials = Credentials::new(
                access_key.to_string(),
                secret_key.to_string(),
                None,
                None,
                "classify-app",
            );
            builder = builder.credentials_provider(credentials);
        }

        let aws_config = builder.load().await;
        let client = S3Client::new(&aws_config);

        match client.head_bucket().bucket(bucket).send().await {
            Ok(_) => {}
            Err(e) => {
                return Err(ClassifyError::StorageError(format!(
                    "Failed to access S3 bucket '{}': {}",
                    bucket, e
                )));
            }
        }

        Ok(Self {
            client,
            bucket: bucket.to_string(),
            prefix: if prefix.ends_with('/') || prefix.is_empty() {
                prefix.to_string()
            } else {
                format!("{}/", prefix)
            },
        })
    }

    fn get_object_key(&self, id: &str) -> String {
        format!("{}{}.json", self.prefix, id)
    }
}

#[async_trait]
impl ContentStorage for S3ContentStorage {
    async fn store(&self, content: &Content) -> ClassifyResult<()> {
        let object_key = self.get_object_key(&content.id.to_string());
        let json =
            serde_json::to_string_pretty(content).map_err(ClassifyError::SerializationError)?;

        let _put_object_response = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .body(ByteStream::from(json.into_bytes()))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| {
                ClassifyError::StorageError(format!("Failed to store content in S3: {}", e))
            })?;

        Ok(())
    }

    async fn get(&self, id: &str) -> ClassifyResult<Option<Content>> {
        let object_key = self.get_object_key(id);

        let get_object_output = match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .send()
            .await
        {
            Ok(output) => output,
            Err(err) => {
                // For 404 errors, return None
                if err.to_string().contains("NoSuchKey") {
                    return Ok(None);
                }
                return Err(ClassifyError::StorageError(format!(
                    "Failed to get content from S3: {}",
                    err
                )));
            }
        };

        let body = get_object_output.body;
        let mut buffer = Vec::new();

        let mut stream = body.into_async_read();
        stream.read_to_end(&mut buffer).await.map_err(|e| {
            ClassifyError::StorageError(format!("Failed to read S3 object body: {}", e))
        })?;

        let content = serde_json::from_slice(&buffer).map_err(ClassifyError::SerializationError)?;

        Ok(Some(content))
    }

    async fn list(&self) -> ClassifyResult<Vec<Content>> {
        let list_objects_output = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&self.prefix)
            .send()
            .await
            .map_err(|e| {
                ClassifyError::StorageError(format!("Failed to list objects in S3: {}", e))
            })?;

        let mut contents = Vec::new();

        if let Some(objects) = list_objects_output.contents() {
            for object in objects {
                if let Some(key) = &object.key {
                    if key.ends_with(".json") && key.starts_with(&self.prefix) {
                        let id = key[self.prefix.len()..key.len() - 5].to_string();
                        if let Some(content) = self.get(&id).await? {
                            contents.push(content);
                        }
                    }
                }
            }
        }

        Ok(contents)
    }

    async fn delete(&self, id: &str) -> ClassifyResult<bool> {
        let object_key = self.get_object_key(id);

        let head_result = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .send()
            .await;

        if let Err(err) = head_result {
            if err.to_string().contains("NotFound") || err.to_string().contains("404") {
                return Ok(false);
            }
            return Err(ClassifyError::StorageError(format!(
                "Failed to check if object exists in S3: {}",
                err
            )));
        }

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .send()
            .await
            .map_err(|e| {
                ClassifyError::StorageError(format!("Failed to delete object from S3: {}", e))
            })?;

        Ok(true)
    }

    async fn find_by_hash(&self, hash: &str) -> ClassifyResult<Option<Content>> {
        // S3 doesn't provide a native way to query objects by their content
        // We need to list all objects and check each one

        let all_content = self.list().await?;

        for content in all_content {
            if content.content_hash.as_deref() == Some(hash) {
                return Ok(Some(content));
            }
        }

        Ok(None)
    }
}
