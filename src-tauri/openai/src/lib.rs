use serde::{Deserialize, Serialize};

const API_KEY: &str = "AIzaSyDBienhzI5Y3o9N4foi7QQRarTtuOIsVJg"; // 请替换为您的OpenAI API密钥
const API_URL: &str = "https://melodic-bonbon-359e24.netlify.app/edge/v1/chat/completions";

// --- Structs for Serialization (Request) ---

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<RequestMessage<'a>>,
}

#[derive(Serialize)]
struct RequestMessage<'a> {
    role: &'a str,
    content: &'a str,
}

// --- Structs for Deserialization (Response) ---

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
}

pub async fn ask_openai(user_prompt: &str, system_prompt: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();

    let messages = vec![
        RequestMessage {
            role: "system",
            content: system_prompt,
        },
        RequestMessage {
            role: "user",
            content: user_prompt,
        },
    ];

    let request_body = ChatCompletionRequest {
        model: "gemini-2.5-flash",
        messages,
    };

    let response = client
        .post(API_URL)
        .bearer_auth(API_KEY)
        .json(&request_body)
        .send()
        .await?;

    let response = response.error_for_status()?;
    let chat_response = response.json::<ChatCompletionResponse>().await?;

    if let Some(choice) = chat_response.choices.get(0) {
        Ok(choice.message.content.clone())
    } else {
        Ok("No response from AI.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ask_openai() {
        // 请注意：此测试会真实调用OpenAI API并产生费用。
        // 请确保您已经设置了正确的API_KEY。
        // 在实际测试中，您可能希望使用模拟（mocking）来避免网络调用。
        let user_prompt = "Hello, who are you?";
        let system_prompt = "You are a helpful assistant.";
        let result = ask_openai(user_prompt, system_prompt).await;
        if let Err(e) = &result {
            eprintln!("API call failed with error: {}", e);
        }
        assert!(result.is_ok());
        println!("AI Response: {}", result.unwrap());
    }
}
