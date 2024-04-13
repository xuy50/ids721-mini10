use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_config::{load_defaults, BehaviorVersion};
use rust_bert::pipelines::sentiment::{SentimentModel, Sentiment, SentimentPolarity};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    if input.command == "sentiment" {
        let sentiment = analyze_sentiment_and_update_s3(&input.text, "ids-721-data").await?;
        Ok(LambdaOutput {
            result: format!("Sentiment: {:?}", sentiment),
        })
    } else {
        Err(LambdaError::InvalidCommand)
    }
}

async fn analyze_sentiment_and_update_s3(text: &str, bucket: &str) -> Result<Sentiment, LambdaError> {
    let sentiment_model = SentimentModel::new(Default::default())
        .map_err(|_| LambdaError::SentimentError)?;
    let sentiments = sentiment_model.predict(&[text]);
    let sentiment = sentiments.into_iter().next().ok_or(LambdaError::SentimentError)?;

    let config = load_defaults(BehaviorVersion::latest()).await;
    let client = S3Client::new(&config);
    update_sentiment_count_in_s3(&client, bucket, &sentiment).await?;

    Ok(sentiment)
}

async fn update_sentiment_count_in_s3(client: &S3Client, bucket: &str, sentiment: &Sentiment) -> Result<(), LambdaError> {
    let key = match sentiment.polarity {
        SentimentPolarity::Positive => "positive-count",
        SentimentPolarity::Negative => "negative-count",
    };

    let get_req = client.get_object().bucket(bucket).key(key).send().await
        .map_err(|_| LambdaError::S3Error)?;

    let body = get_req.body.collect().await
        .map_err(|_| LambdaError::S3Error)?;

    let count_data = String::from_utf8(body.into_bytes().to_vec())
        .map_err(|_| LambdaError::InternalError("UTF-8 conversion failed".to_string()))?;
    let mut count: i32 = count_data.parse()
        .map_err(|_| LambdaError::InternalError("Count parsing error".to_string()))?;

    count += 1;
    let new_count_data = count.to_string();
    let body = ByteStream::from(new_count_data.into_bytes());

    client.put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send().await
        .map_err(|_| LambdaError::S3Error)?;

    Ok(())
}


//test
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_event_positive_sentiment() -> Result<(), LambdaError> {
        let input = LambdaInput {
            command: "sentiment".to_string(),
            text: "I love Rust!".to_string(),
        };
        let output = process_input(input).await?;
        assert!(output.result.contains("Positive"));
        Ok(())
    }

    #[tokio::test]
    async fn test_process_event_negative_sentiment() -> Result<(), LambdaError> {
        let input = LambdaInput {
            command: "sentiment".to_string(),
            text: "I hate Rust!".to_string(),
        };
        let output = process_input(input).await?;
        assert!(output.result.contains("Negative"));
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_command() -> Result<(), LambdaError> {
        let input = LambdaInput {
            command: "unknown_command".to_string(),
            text: "Some text".to_string(),
        };
        let result = process_input(input).await;
        assert!(matches!(result, Err(LambdaError::InvalidCommand)));
        Ok(())
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tokio::runtime::Runtime;

//     #[test]
//     fn test_process_event_positive_sentiment() -> Result<(), LambdaError> {
//         let rt = Runtime::new().unwrap();
//         rt.block_on(async {
//             let input = LambdaInput {
//                 command: "sentiment".to_string(),
//                 text: "I love Rust!".to_string(),
//             };
//             let output = process_input(input).await?;
//             assert!(output.result.contains("Positive"));
//             Ok(())
//         })
//     }

    // #[test]
    // fn test_process_event_negative_sentiment() -> Result<(), LambdaError> {
    //     let rt = Runtime::new().unwrap();
    //     rt.block_on(async {
    //         let input = LambdaInput {
    //             command: "sentiment".to_string(),
    //             text: "I hate Rust!".to_string(),
    //         };
    //         let output = process_input(input).await?;
    //         assert!(output.result.contains("Negative"));
    //         Ok(())
    //     })
    // }

    // #[test]
    // fn test_invalid_command() -> Result<(), LambdaError> {
    //     let rt = Runtime::new().unwrap();
    //     rt.block_on(async {
    //         let input = LambdaInput {
    //             command: "unknown_command".to_string(),
    //             text: "Some text".to_string(),
    //         };
    //         let result = process_input(input).await;
    //         assert!(matches!(result, Err(LambdaError::InvalidCommand)));
    //         Ok(())
    //     })
    // }
// }
