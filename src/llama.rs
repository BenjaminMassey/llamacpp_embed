#[derive(serde::Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}
#[derive(serde::Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(serde::Deserialize)]
struct Choice {
    message: ResponseMessage,
}
#[derive(serde::Deserialize)]
struct ResponseMessage {
    content: String,
}

pub fn is_ready() -> bool {
    if let Ok(resp) = reqwest::blocking::get("http://localhost:8080/health") {
        let json: Result<serde_json::Value, _> = resp.json();
        if let Ok(data) = json {
            return data["status"] == "ok";
        }
    }
    false
}

pub fn chat(
    system_message: &str,
    user_message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();

    let request = ChatRequest {
        model: "default".to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_message.to_owned(),
            },
            Message {
                role: "user".to_string(),
                content: user_message.to_owned(),
            },
        ],
    };

    let chat_response: ChatResponse = client
        .post("http://localhost:8080/v1/chat/completions")
        .json(&request)
        .send()?
        .json()?;

    if chat_response.choices.is_empty() {
        return Err("Server returned no choices.".into());
    }

    Ok(chat_response.choices[0].message.content.clone())
}
