// use aws_sdk_s3::operation::get_object::GetObjectError;
// use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use aws_config::{load_defaults, BehaviorVersion};
use rust_bert::pipelines::sentiment::{SentimentModel, Sentiment, SentimentPolarity};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use aws_smithy_runtime_api::client::result::SdkError;
use std::string::FromUtf8Error;
use aws_sdk_s3::primitives::ByteStreamError;
use rust_bert::RustBertError;

// Define a custom error type with From implementations for common errors
#[derive(Error, Debug)]
pub enum LambdaError {
    #[error("Invalid command")]
    InvalidCommand,
    #[error("Sentiment analysis error: {0}")]
    SentimentError(Box<dyn std::error::Error + Send + Sync>),
    #[error("S3 error: {0}")]
    S3Error(Box<dyn std::error::Error + Send + Sync>),
    #[error("RustBert error: {0}")]
    RustBertError(#[from] RustBertError),
    #[error("ByteStream Error: {0}")]
    ByteStreamError(#[from] ByteStreamError),
    #[error("UTF-8 Encoding Error: {0}")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("Count Parsing Error: {0}")]
    Other(String),
}

#[derive(Deserialize)]
pub struct LambdaInput {
    pub command: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct LambdaOutput {
    pub result: String,
}

pub async fn process_input(input: LambdaInput) -> Result<LambdaOutput, LambdaError> {
    match input.command.as_str() {
        "sentiment" => {
            let sentiment = analyze_sentiment_and_update_s3(&input.text, "your-bucket-name").await?;
            Ok(LambdaOutput {
                result: format!("Sentiment: {:?}", sentiment),
            })
        }
        _ => Err(LambdaError::InvalidCommand),
    }
}

async fn analyze_sentiment_and_update_s3(text: &str, bucket: &str) -> Result<Sentiment, LambdaError> {
    let sentiment_model = SentimentModel::new(Default::default())?;
    let mut sentiments = sentiment_model.predict(&[text]);
    let sentiment = sentiments.pop().unwrap(); // Use pop() to take ownership

    let config = load_defaults(BehaviorVersion::latest()).await;
    let client = S3Client::new(&config);

    update_sentiment_count_in_s3(&client, bucket, &sentiment).await?;

    Ok(sentiment)
}

impl<T> From<SdkError<T, ()>> for LambdaError 
    where T: std::error::Error + Send + Sync + 'static 
{
    fn from(err: SdkError<T, ()>) -> Self {
        LambdaError::S3Error(Box::new(err))
    }
}

async fn update_sentiment_count_in_s3(client: &S3Client, bucket: &str, sentiment: &Sentiment) -> Result<(), LambdaError> {
    let key = match sentiment.polarity {
        SentimentPolarity::Positive => "positive-count",
        SentimentPolarity::Negative => "negative-count",
    };

    // Handle S3 errors with conversions
    let get_req = client.get_object().bucket(bucket).key(key).send().await?;
    let body = get_req.body.collect().await
        .map_err(|err| LambdaError::ByteStreamError(err))?;
    let count_data = String::from_utf8(body.into_bytes().to_vec())
        .map_err(|err| LambdaError::FromUtf8Error(err))?;
    let mut count: i32 = count_data.parse()
        .map_err(|err| LambdaError::Other("Count Parsing Error".into()))?;

    count += 1;

  let new_count_data = count.to_string();
  let body = ByteStream::from(new_count_data.into_bytes());

  client.put_object()
    .bucket(bucket)
    .key(key)
    .body(body)
    .send()
    .await?;

  Ok(())
}
