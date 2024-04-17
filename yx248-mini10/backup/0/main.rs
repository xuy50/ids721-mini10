use tracing::{Level};
use tracing_subscriber::FmtSubscriber;
use tracing_subscriber::filter::EnvFilter;
use lambda_http::{run, service_fn, Body, Error, Request, Response, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::error::SdkError;
use aws_config::{self, Region};
use rust_bert::pipelines::sentiment::{SentimentModel, Sentiment, SentimentPolarity};
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;
use csv::{ReaderBuilder, WriterBuilder};

#[derive(Error, Debug)]
pub enum LambdaError {
    #[error("Invalid command")]
    InvalidCommand,
    #[error("Sentiment analysis error")]
    SentimentError,
    #[error("AWS S3 Error")]
    S3Error,
    #[error("Internal Error: {0}")]
    InternalError(String),
}

#[derive(Deserialize, Serialize)]
pub struct LambdaInput {
    pub command: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct LambdaOutput {
    pub result: String,
}

async fn process_input(input: LambdaInput, sentiment_model: Arc<Mutex<SentimentModel>>) -> Result<LambdaOutput, LambdaError> {
    if input.command == "sentiment" {
        // 使用传入的sentiment_model进行情绪分析
        let sentiment = analyze_sentiment_and_update_s3(&input.text, "ids-721-data", sentiment_model).await?;
        Ok(LambdaOutput {
            result: format!("Sentiment: {:?}", sentiment),
        })
    } else {
        Err(LambdaError::InvalidCommand)
    }
}

// async fn analyze_sentiment_and_update_s3(text: &str, bucket: &str, sentiment_model: Arc<Mutex<SentimentModel>>) -> Result<Sentiment, LambdaError> {
//     // 获取Mutex的锁
//     let model = sentiment_model.lock().await;
//     let sentiments = model.predict(&[text]);
//     let sentiment = sentiments.into_iter().next().ok_or(LambdaError::SentimentError)?;

//     let config = aws_config::from_env().region(Region::new("us-east-2")).load().await;
//     let client = S3Client::new(&config);
//     update_sentiment_count_in_s3(&client, bucket, &sentiment).await?;

//     Ok(sentiment)
// }

// async fn update_sentiment_count_in_s3(client: &S3Client, bucket: &str, sentiment: &Sentiment) -> Result<(), LambdaError> {
//     let key = match sentiment.polarity {
//         SentimentPolarity::Positive => "positive-count",
//         SentimentPolarity::Negative => "negative-count",
//     };

//     let get_req = client.get_object().bucket(bucket).key(key).send().await
//         .map_err(|_| LambdaError::S3Error)?;

//     let body = get_req.body.collect().await
//         .map_err(|_| LambdaError::S3Error)?;

//     let count_data = String::from_utf8(body.into_bytes().to_vec())
//         .map_err(|_| LambdaError::InternalError("UTF-8 conversion failed".to_string()))?;
//     let mut count: i32 = count_data.parse()
//         .map_err(|_| LambdaError::InternalError("Count parsing error".to_string()))?;

//     count += 1;
//     let new_count_data = count.to_string();
//     let body = ByteStream::from(new_count_data.into_bytes());

//     client.put_object().bucket(bucket).key(key).body(body).send().await
//         .map_err(|_| LambdaError::S3Error)?;

//     Ok(())
// }

fn convert_s3_error(e: aws_sdk_s3::types::SdkError<aws_sdk_s3::error::GetObjectError>) -> LambdaError {
    LambdaError::S3Error(format!("{:?}", e)) // 更详细地记录错误
}

async fn analyze_sentiment_and_update_s3(text: &str, bucket: &str, sentiment_model: Arc<Mutex<SentimentModel>>, client: &S3Client) -> Result<Sentiment, LambdaError> {
    // 获取模型并进行情绪分析
    let model = sentiment_model.lock().await;
    let sentiments = model.predict(&[text]);
    let sentiment = sentiments.into_iter().next().ok_or(LambdaError::SentimentError)?;

    // 读取 CSV 文件
    let get_req = client.get_object().bucket(bucket).key("sentiment.csv").send().await
        .map_err(convert_s3_error)?;

    let body = get_req.body.collect().await
        .map_err(|_| LambdaError::InternalError("Failed to collect body".to_string()))?;
    let data = String::from_utf8(body.into_bytes().to_vec())
        .map_err(|_| LambdaError::InternalError("UTF-8 conversion failed".to_string()))?;

    // 解析 CSV 数据
    let mut rdr = ReaderBuilder::new().from_reader(data.as_bytes());
    let mut records: Vec<(String, i32)> = Vec::new();
    for result in rdr.deserialize() {
        let record: (String, i32) = result.map_err(|_| LambdaError::InternalError("CSV parsing error".to_string()))?;
        records.push(record);
    }

    // 更新情绪计数
    for record in records.iter_mut() {
        if record.0 == "positive" && sentiment.polarity == SentimentPolarity::Positive {
            record.1 += 1;
        } else if record.0 == "negative" && sentiment.polarity == SentimentPolarity::Negative {
            record.1 += 1;
        }
    }

    // 将更新后的数据写回 CSV
    let mut wtr = WriterBuilder::new().from_writer(Vec::new());
    for record in &records {
        wtr.serialize(record).map_err(|_| LambdaError::InternalError("CSV writing error".to_string()))?;
    }
    let data = String::from_utf8(wtr.into_inner().map_err(|_| LambdaError::InternalError("Failed to finalize CSV".to_string()))?)
        .map_err(|_| LambdaError::InternalError("UTF-8 conversion failed".to_string()))?;

    // 上传更新后的 CSV 文件
    let body = ByteStream::from(data.into_bytes());
    client.put_object().bucket(bucket).key("sentiment.csv").body(body).send().await
        .map_err(convert_s3_error)?;

    Ok(sentiment)
}

async fn function_handler(event: Request, sentiment_model: Arc<Mutex<SentimentModel>>) -> Result<Response<Body>, Error> {
    if let Ok(input) = serde_json::from_slice::<LambdaInput>(event.body()) {
        // 调用process_input，传递input和sentiment_model
        let output = process_input(input, sentiment_model).await.map_err(|e| {
            LambdaError::InternalError(format!("Failed to process input: {}", e))
        })?;

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(json!(output).to_string().into())
            .expect("Failed to render response"))
    } else {
        Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body("Invalid request".into())
            .expect("Failed to render response"))
    }
}

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::builder()
            .with_default_directive(Level::INFO.into())
            .from_env_lossy())
        .with_target(false)
        .without_time()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");

    // 使用block_in_place加载模型
    let sentiment_model = tokio::task::block_in_place(|| {
        SentimentModel::new(Default::default()).expect("Failed to load the sentiment model")
    });
    let sentiment_model = Arc::new(Mutex::new(sentiment_model));

    run(service_fn(move |req| function_handler(req, Arc::clone(&sentiment_model)))).await
}
