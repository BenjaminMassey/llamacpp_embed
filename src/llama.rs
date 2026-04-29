use crate::LlamaEmbedChat;
use crate::Message;
use base64::Engine;

pub fn is_ready() -> bool {
    if let Ok(resp) = reqwest::blocking::get("http://localhost:8080/health") {
        let json: Result<serde_json::Value, _> = resp.json();
        if let Ok(data) = json {
            return data["status"] == "ok";
        }
    }
    false
}

#[derive(serde::Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
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
pub fn chat(
    system_message: &str,
    user_message: &str,
    prev_messages: Option<&[Message]>,
) -> Result<LlamaEmbedChat, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();

    let mut chat_messages = if let Some(messages) = prev_messages {
        messages.to_owned()
    } else {
        vec![]
    };
    chat_messages.push(Message {
        role: "user".to_string(),
        content: user_message.to_owned(),
    });

    let mut all_messages = vec![Message {
        role: "system".to_string(),
        content: system_message.to_owned(),
    }];
    all_messages.append(&mut chat_messages.clone());

    let request = ChatRequest {
        model: "default".to_string(),
        messages: all_messages.clone(),
    };

    let chat_response: ChatResponse = client
        .post("http://localhost:8080/v1/chat/completions")
        .json(&request)
        .send()?
        .json()?;

    if chat_response.choices.is_empty() {
        return Err("Server returned no choices.".into());
    }

    Ok(LlamaEmbedChat {
        response: chat_response.choices[0].message.content.clone(),
        messages: chat_messages,
    })
}

#[derive(serde::Serialize)]
struct VisionChatRequest {
    model: String,
    messages: Vec<VisionMessage>,
}
#[derive(serde::Serialize)]
#[serde(untagged)]
enum VisionMessage {
    Text {
        role: String,
        content: String,
    },
    Multi {
        role: String,
        content: Vec<ContentPart>,
    },
}
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}
#[derive(serde::Serialize)]
struct ImageUrl {
    url: String,
}
pub fn chat_with_image(
    system_message: &str,
    user_message: &str,
    image_path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();

    let image_bytes = std::fs::read(image_path)?;
    let mime = match std::path::Path::new(image_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("jpg") | Some("jpeg") | _ => "image/jpeg",
    };

    let b64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
    let data_url = format!("data:{};base64,{}", mime, b64);

    let request = VisionChatRequest {
        model: "default".to_string(),
        messages: vec![
            VisionMessage::Text {
                role: "system".to_string(),
                content: system_message.to_owned(),
            },
            VisionMessage::Multi {
                role: "user".to_string(),
                content: vec![
                    ContentPart::Text {
                        text: user_message.to_owned(),
                    },
                    ContentPart::ImageUrl {
                        image_url: ImageUrl { url: data_url },
                    },
                ],
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
