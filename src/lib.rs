mod llama;

pub use self::llama::ContentPart;
pub use self::llama::ImageUrl;
pub use self::llama::LlamaEmbedChat;
pub use self::llama::LlamaEmbedImageChat;
pub use self::llama::Message;
pub use self::llama::VisionMessage;
pub use self::llama::image_bytes_to_url;
pub use self::llama::image_path_to_url;

pub struct LlamaEmbedModel {
    program: std::process::Child,
    system_prompt: String,
    image_capable: bool,
    port: String,
}
pub fn start(
    gguf_path: &str,
    mmproj_path: Option<&str>,
    system_prompt: &str,
    load_timeout: u64,
    reasoning_budget: Option<u64>,
    server_port: Option<u64>,
) -> Result<LlamaEmbedModel, Box<dyn std::error::Error>> {
    if !std::path::Path::new(gguf_path).exists() {
        return Err(format!("Model not found: \"{}\".", gguf_path).into());
    }

    let port = if let Some(given_port) = server_port {
        given_port
    } else {
        8080
    }
    .to_string();

    let mut args = vec!["-m", gguf_path, "--port", &port];
    if let Some(mmproj) = mmproj_path {
        args.append(&mut vec!["--mmproj", mmproj]);
    }
    let budget_str;
    if let Some(budget) = reasoning_budget {
        budget_str = budget.to_string();
        args.append(&mut vec!["--reasoning-budget", &budget_str]);
    }

    let log = std::fs::File::create("llamacpp_log.txt").unwrap();
    let program = std::process::Command::new(
        &std::path::Path::new(&llama::server_path())
            .to_str()
            .unwrap(),
    )
    .args(&args)
    .stdout(log.try_clone().unwrap())
    .stderr(log)
    .spawn()?;

    let load_start = std::time::Instant::now();
    while !llama::is_ready() {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if std::time::Instant::now()
            .duration_since(load_start)
            .as_secs()
            >= load_timeout
        {
            return Err(format!("Reached \"load_timeout\" of {}.", load_timeout).into());
        }
    }

    Ok(LlamaEmbedModel {
        program,
        system_prompt: system_prompt.to_owned(),
        image_capable: mmproj_path.is_some(),
        port,
    })
}

pub fn chat(
    model: &mut LlamaEmbedModel,
    prompt: &str,
    prev_messages: Option<&[Message]>,
) -> Result<LlamaEmbedChat, Box<dyn std::error::Error>> {
    llama::chat(&model.system_prompt, prompt, prev_messages, &model.port)
}

pub fn chat_with_image_path(
    model: &mut LlamaEmbedModel,
    prompt: &str,
    image_path: &std::path::Path,
    prev_messages: Option<&[VisionMessage]>,
) -> Result<LlamaEmbedImageChat, Box<dyn std::error::Error>> {
    if !model.image_capable {
        return Err("llamacpp_embed::start(..) was not provided with an MMPROJ file.".into());
    }
    llama::chat_with_image(
        &model.system_prompt,
        prompt,
        llama::image_path_to_url(image_path),
        prev_messages,
        &model.port,
    )
}

pub fn chat_with_image_bytes(
    model: &mut LlamaEmbedModel,
    prompt: &str,
    image_bytes: &[u8],
    mime_type: &str,
    prev_messages: Option<&[VisionMessage]>,
) -> Result<LlamaEmbedImageChat, Box<dyn std::error::Error>> {
    if !model.image_capable {
        return Err("llamacpp_embed::start(..) was not provided with an MMPROJ file.".into());
    }
    llama::chat_with_image(
        &model.system_prompt,
        prompt,
        llama::image_bytes_to_url(image_bytes, mime_type),
        prev_messages,
        &model.port,
    )
}

pub fn stop(model: &mut LlamaEmbedModel) -> Result<(), Box<dyn std::error::Error>> {
    model.program.kill()?;
    model.program.wait()?;
    Ok(())
}
