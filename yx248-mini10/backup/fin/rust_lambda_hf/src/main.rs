use tracing::{Level};
use tracing_subscriber::FmtSubscriber;
use tracing_subscriber::filter::EnvFilter;
use lambda_http::{run, service_fn, Body, Error, Request, Response, http, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_urlencoded;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_config::{self, load_defaults, BehaviorVersion};
use csv::{ReaderBuilder, WriterBuilder};
use rust_bert::pipelines::sentiment::{SentimentModel, Sentiment};
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;
use std::collections::HashMap;

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
        let sentiment = analyze_sentiment_and_update_s3(&input.text, "sentiments-data", sentiment_model).await?;
        Ok(LambdaOutput {
            result: format!("Sentiment: {:?}", sentiment),
        })
    } else {
        Err(LambdaError::InvalidCommand)
    }
}

async fn analyze_sentiment_and_update_s3(text: &str, bucket: &str, sentiment_model: Arc<Mutex<SentimentModel>>) -> Result<Sentiment, LambdaError> {
    // 获取Mutex的锁
    let model = sentiment_model.lock().await;
    let sentiments = model.predict(&[text]);
    let sentiment = sentiments.into_iter().next().ok_or(LambdaError::SentimentError)?;

    let config = load_defaults(BehaviorVersion::latest()).await;
    let s3_client = S3Client::new(&config);
    update_sentiment_count_in_s3(&s3_client, bucket, &sentiment).await?;

    Ok(sentiment)
}

#[derive(Serialize, Deserialize, Debug)]
struct SentimentRecord {
    #[serde(rename = "Sentiment")]
    sentiment: String,
    #[serde(rename = "Count")]
    count: i32,
}

async fn read_and_parse_csv(client: &S3Client, bucket: &str, key: &str) -> Result<Vec<SentimentRecord>, LambdaError> {
    let get_req = client.get_object()
                        .bucket(bucket)
                        .key(key)
                        .send()
                        .await
                        .map_err(|_| LambdaError::S3Error)?;

    let bytes_stream = get_req
        .body
        .collect()
        .await
        .map_err(|_| LambdaError::S3Error)?;

    let bytes = bytes_stream.into_bytes();

    let csv_content = std::str::from_utf8(&bytes)
        .map_err(|_| LambdaError::InternalError("Invalid UTF-8 sequence".into()))?;

    let mut rdr = ReaderBuilder::new().from_reader(csv_content.as_bytes());
    let records: Result<Vec<SentimentRecord>, csv::Error> = rdr.deserialize().collect();
    records.map_err(|_| LambdaError::InternalError("Failed to parse CSV".into()))
}

async fn update_sentiment_count_in_s3(client: &S3Client, bucket: &str, update_sentiment: &Sentiment) -> Result<(), LambdaError> {
    let key = "sentiment.csv";
    let records = read_and_parse_csv(client, bucket, key).await?;
    // println!("Read records: {:?}", records);  // check if records are read correctly

    // 更新情感计数
    let mut map = records.into_iter().fold(HashMap::new(), |mut acc, rec| {
        // acc.entry(rec.sentiment).or_insert(rec.count);
        *acc.entry(rec.sentiment).or_insert(0) += rec.count; // 注意这里需要进行累加
        acc
    });
    // 定义更新键的变量
    let key_to_update = format!("{:?}", update_sentiment.polarity);
    // println!("Trying to access key: {:?}", key_to_update);
    // println!("Trying to access key: {:?}", format!("{:?}", update_sentiment.polarity)); // check map key

    // *map.get_mut(&format!("{:?}", update_sentiment.polarity)).unwrap() += 1;
    *map.entry(key_to_update).or_insert(0) += 1; // 直接使用 entry API 安全更新或插入

    // println!("Updated map: {:?}", map);  // check if map is updated correctly

    // 将更新后的数据写回 CSV
    let mut wtr = Vec::new();

    {
        let mut csv_writer = WriterBuilder::new().from_writer(&mut wtr);
        for (sentiment, count) in &map {
            csv_writer.serialize(SentimentRecord { sentiment: sentiment.clone(), count: *count })
                .map_err(|_| LambdaError::InternalError("Failed to write CSV".into()))?;
        }

        // Crucial step: Flush the writer
        csv_writer.flush().map_err(|_| LambdaError::InternalError("Failed to flush CSV".into()))?;
    }

    // Now you can safely move the contents of 'wtr'
    let data = wtr;

    // 写回到 S3
    client.put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(data))
        .send()
        .await
        .map_err(|_| LambdaError::S3Error)?;

    Ok(())
}

async fn function_handler(event: Request, sentiment_model: Arc<Mutex<SentimentModel>>) -> Result<Response<Body>, Error> {
    // 确保请求是 GET 请求
    if event.method() == http::Method::GET {
        // 解析 URL 查询参数
        let query_params = event.uri().query().unwrap_or("");
        let query_map: Result<HashMap<String, String>, _> = serde_urlencoded::from_str(query_params);

        if let Err(_) = query_map {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(json!({"error": "Invalid query parameters"}).to_string().into())
                .expect("Failed to render response"));
        }

        // 从查询参数中提取 'text' 字段
        if let Some(text) = query_map.unwrap().get("text") {
            let input = LambdaInput {
                command: "sentiment".to_string(),
                text: text.clone(),
            };
            let output = process_input(input, sentiment_model).await.map_err(|e| {
                LambdaError::InternalError(format!("Failed to process input: {}", e))
            })?;

            println!("{:?}", json!(output).to_string());
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(json!(output).to_string().into())
                .expect("Failed to render response"))
        } else {
            // 如果没有提供 text 参数，返回错误响应
            println!("error: Missing text parameter. Failed to render response.");
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(json!({"error": "Missing text parameter"}).to_string().into())
                .expect("Failed to render response"));
        }
    } else {
        // 如果不是 GET 请求，返回方法不允许
        println!("error: Method not allowed. Failed to render response.");
        Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header("Content-Type", "application/json")
            .body(json!({"error": "Method not allowed"}).to_string().into())
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
