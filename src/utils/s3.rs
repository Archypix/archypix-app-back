use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use crate::utils::thumbnail::PictureThumbnail;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use aws_smithy_types::byte_stream::ByteStream;
use std::env;
use std::path::Path;
use std::time::Duration;

/// Should match the thumbnails type in utils::thumbnail::PictureThumbnail
const BUCKETS: [&str; 4] = [
    "archypix-pictures",
    "archypix-thumbnails-small",
    "archypix-thumbnails-medium",
    "archypix-thumbnails-large",
];

pub struct PictureStorer {
    client: Client,
}
impl PictureStorer {
    pub async fn new() -> Self {
        let mut config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .force_path_style(true)
            .region(aws_sdk_s3::config::Region::new(
                env::var("AWS_REGION").expect("Missing AWS_REGION environment variable"),
            ))
            .credentials_provider(Credentials::new(
                env::var("AWS_ACCESS_KEY_ID").unwrap(),
                env::var("AWS_SECRET_ACCESS_KEY").unwrap(),
                None,
                None,
                "Static",
            ));
        if let Some(endpoint) = env::var("AWS_ENDPOINT").ok() {
            config_builder = config_builder.endpoint_url(endpoint)
        }
        let config = config_builder.build();
        let client = Client::from_conf(config);

        // Test connection
        client.list_buckets().send().await.expect("Unable to connect to S3");

        let picture_storer = PictureStorer { client };
        picture_storer.create_buckets().await;
        picture_storer
    }
    async fn create_buckets(&self) {
        let list_buckets_output = self.client.list_buckets().send().await.unwrap();
        let existing_bucket_names: Vec<String> = list_buckets_output
            .buckets()
            .iter()
            .map(|bucket| bucket.name().unwrap_or_default().to_string())
            .collect();

        for bucket_name in BUCKETS.iter() {
            if !existing_bucket_names.contains(&bucket_name.to_string()) {
                let create_bucket_output = self.client.create_bucket().bucket(bucket_name.to_string()).send().await.unwrap();
                info!("Created bucket: {:?}", create_bucket_output);
            } else {
                info!("Bucket '{}' already exists.", bucket_name);
            }
        }
    }

    pub async fn store_picture_from_file(&self, picture_thumbnail: PictureThumbnail, id: u64, path: &Path) -> Result<(), ErrorResponder> {
        self.client
            .put_object()
            .bucket(BUCKETS[picture_thumbnail as usize])
            .key(id.to_string())
            .body(
                ByteStream::from_path(path)
                    .await
                    .map_err(|_e| ErrorType::S3Error(String::from("Unable to read file")).res())?,
            )
            .send()
            .await
            .map(|_| ())
            .map_err(|_e| ErrorType::S3Error(String::from("Unable to store object")).res())
    }

    pub async fn get_picture(&self, picture_thumbnail: PictureThumbnail, id: u64) -> Result<ByteStream, ErrorResponder> {
        self.client
            .get_object()
            .bucket(BUCKETS[picture_thumbnail as usize])
            .key(id.to_string())
            .send()
            .await
            .map(|output| output.body)
            .map_err(|_e| ErrorType::S3Error(String::from("Unable to retrieve object")).res())
    }

    pub async fn get_picture_as_url(&self, picture_thumbnail: PictureThumbnail, id: u64) -> Result<String, ErrorResponder> {
        self.client
            .get_object()
            .bucket(BUCKETS[picture_thumbnail as usize])
            .key(id.to_string())
            .presigned(
                PresigningConfig::builder()
                    .expires_in(Duration::from_secs(60 * 5))
                    .build()
                    .expect("Unable to build presigning config"),
            )
            .await
            .map(|output| String::from(output.uri()))
            .map_err(|_e| ErrorType::S3Error(String::from("Unable to retrieve object")).res())
    }
}
