# yx248-mini10

Rust Lambda Hugging Face Transformer

This repository contains the code and deployment instructions for a sentiment analysis Lambda function, built with Rust and the Hugging Face's transformers library, containerized with Docker, and deployed to AWS Lambda.


## Project Structure

```bash
rust_lambda_hf
│
├── src
│ └── main.rs
│
├── Cargo.toml
│
├── Dockerfile
│
└── .gitignore
```

## Features

### Sentiment Analysis API
The application includes a sentiment analysis API that processes text inputs to determine the sentiment polarity (positive or negative) and intensity score. The model is served using an AWS Lambda function exposed through a query endpoint.

### Query Endpoint
The `/` endpoint accepts GET requests with a query parameter text containing the text to be analyzed. The sentiment analysis is performed using a pre-trained Hugging Face transformer model, and the results are updated in an AWS S3 bucket.

### Example Request:

```bash
curl "http://localhost:9000/?text=I%20love%20rust"
```

```bash
curl "https://34ct5jlwn1.execute-api.us-east-2.amazonaws.com/mini10-rust-hf-lambda?text=I%20love%20rust"
```

### Example Response:

```json
{"result":"Sentiment: Sentiment { polarity: Negative, score: 0.9178980588912964 }"}
```

This response indicates that the sentiment of the input text is negative with a high confidence score. Note that the polarity might not always align perfectly with the input text as it depends on the underlying model's interpretation.

## Deployment Process

### Dockerizing the Application

The application is containerized using Docker. The Docker image is then pushed to an Amazon Elastic Container Registry (ECR).

``` bash
aws ecr get-login-password --region us-east-2 | docker login --username AWS --password-stdin <ECR-url>

docker build -t <docker-name> .

docker tag <docker-name>:<ECR-repo-url>:latest

docker push <ECR-repo-url>:latest
```

ECR Repository:

![ECR Repository](images/ECR-repo.png)

Docker Image in ECR:

![Docker Image in ECR](images/ECR-image.png)

Docker Push Command Output:

![Docker Push Command Output](images/docker_pushing.png)
![Docker Push Command Output](images/docker_push.png)

### AWS Lambda Deployment

The Docker image is deployed as an AWS Lambda function, leveraging AWS's native capabilities to run containerized applications.

Lambda Functions Dashboard:

![Lambda Functions Dashboard](images/lambda_package_type.png)

Function Overview:

![Function Overview](images/mini10-deployed.png)

### Local Testing

Before deploying to AWS Lambda, the function can be tested locally to ensure its behavior:

![Local Test Run](images/local-run-test.png)

### AWS Test Result

After deploying to AWS Lambda, the function can be invoked and tested to confirm successful deployment and operation:

![AWS Test Result](images/aws-test-result.png)

