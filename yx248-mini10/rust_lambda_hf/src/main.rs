use lambda_runtime::{handler_fn, Context, Error};
use serde_json::Value;
use rust_lambda_hf::{process_input, LambdaError, LambdaInput, LambdaOutput};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(process_event);
    lambda_runtime::run(func).await?;
    Ok(())
}

// async fn process_event(event: Value, _: Context) -> Result<Value, Error> {
//     let input: LambdaInput = serde_json::from_value(event)?;
//     let output = rust_lambda_hf::process_input(input).await.map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;
//     let output_value = serde_json::to_value(output)?;
//     Ok(output_value)
// }

async fn process_event(event: Value, _: Context) -> Result<Value, Error> {
    let input: LambdaInput = serde_json::from_value(event)?;

    // Error handling and conversion
    let output = match rust_lambda_hf::process_input(input).await {
        Ok(output) => output,
        Err(err) => {
            // Log or report the error more thoroughly here
            return Err(Box::new(LambdaError::from(err)));
        }
    };

    Ok(serde_json::to_value(output)?)
}