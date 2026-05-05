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

pub struct LlamaEmbedBuilder {
    gguf_path: String,
    mmproj_path: Option<String>,
    system_prompt: String,
    load_timeout: u64,
    reasoning_budget: Option<u64>,
    server_port: u64,
    parallel_count: Option<u64>,
    context_size: Option<u64>,
}

impl LlamaEmbedBuilder {
    pub fn new(gguf_path: &str) -> Self {
        LlamaEmbedBuilder {
            gguf_path: gguf_path.to_owned(),
            mmproj_path: None,
            system_prompt: "You are a helpful asssitant.".to_owned(),
            load_timeout: 60,
            reasoning_budget: None,
            server_port: 8080,
            parallel_count: None,
            context_size: None,
        }
    }

    pub fn with_mmproj(mut self, mmproj_path: &str) -> Self {
        self.mmproj_path = Some(mmproj_path.to_owned());
        self
    }

    pub fn with_system_prompt(mut self, system_prompt: &str) -> Self {
        self.system_prompt = system_prompt.to_owned();
        self
    }

    pub fn with_load_timeout(mut self, load_timeout: u64) -> Self {
        self.load_timeout = load_timeout;
        self
    }

    pub fn with_reasoning_budget(mut self, reasoning_budget: u64) -> Self {
        self.reasoning_budget = Some(reasoning_budget);
        self
    }

    pub fn with_port(mut self, port: u64) -> Self {
        self.server_port = port;
        self
    }

    pub fn with_parallel(mut self, parallel_count: u64) -> Self {
        self.parallel_count = Some(parallel_count);
        self
    }

    pub fn with_context_size(mut self, context_size: u64) -> Self {
        self.context_size = Some(context_size);
        self
    }

    pub fn build(self) -> Result<LlamaEmbedModel, Box<dyn std::error::Error>> {
        if !std::path::Path::new(&self.gguf_path).exists() {
            return Err(format!("Model not found: \"{}\".", self.gguf_path).into());
        }

        let port = self.server_port.to_string();
        let mut args = vec!["-m", &self.gguf_path, "--port", &port];

        let mmproj_str = self.mmproj_path.as_deref().unwrap_or_default().to_owned();
        if self.mmproj_path.is_some() {
            args.append(&mut vec!["--mmproj", &mmproj_str]);
        }

        let budget_str;
        if let Some(budget) = self.reasoning_budget {
            budget_str = budget.to_string();
            args.append(&mut vec!["--reasoning-budget", &budget_str]);
        }

        let parallel_str;
        if let Some(parallel) = self.parallel_count {
            parallel_str = parallel.to_string();
            args.append(&mut vec!["-np", &parallel_str]);
        }

        let context_str;
        if let Some(context) = self.context_size {
            context_str = context.to_string();
            args.append(&mut vec!["-c", &context_str]);
        }

        let log = std::fs::File::create("llamacpp_log.txt").unwrap();
        let program = std::process::Command::new(
            std::path::Path::new(&llama::server_path())
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
                >= self.load_timeout
            {
                return Err(format!("Reached \"load_timeout\" of {}.", self.load_timeout).into());
            }
        }

        Ok(LlamaEmbedModel {
            program,
            system_prompt: self.system_prompt,
            image_capable: self.mmproj_path.is_some(),
            port,
        })
    }
}

pub fn chat(
    model: &mut LlamaEmbedModel,
    prompt: &str,
    prev_messages: Option<&[Message]>,
    id_slot: Option<u64>,
) -> Result<LlamaEmbedChat, Box<dyn std::error::Error>> {
    llama::chat(
        &model.system_prompt,
        prompt,
        prev_messages,
        &model.port,
        id_slot,
    )
}

pub fn chat_with_image_path(
    model: &mut LlamaEmbedModel,
    prompt: &str,
    image_path: &std::path::Path,
    prev_messages: Option<&[VisionMessage]>,
    id_slot: Option<u64>,
) -> Result<LlamaEmbedImageChat, Box<dyn std::error::Error>> {
    if !model.image_capable {
        return Err(
            "llamacpp_embed::LlamaEmbedBuilder was not provided with an MMPROJ file.".into(),
        );
    }
    llama::chat_with_image(
        &model.system_prompt,
        prompt,
        llama::image_path_to_url(image_path),
        prev_messages,
        &model.port,
        id_slot,
    )
}

pub fn chat_with_image_bytes(
    model: &mut LlamaEmbedModel,
    prompt: &str,
    image_bytes: &[u8],
    mime_type: &str,
    prev_messages: Option<&[VisionMessage]>,
    id_slot: Option<u64>,
) -> Result<LlamaEmbedImageChat, Box<dyn std::error::Error>> {
    if !model.image_capable {
        return Err(
            "llamacpp_embed::LlamaEmbedBuilder was not provided with an MMPROJ file.".into(),
        );
    }
    llama::chat_with_image(
        &model.system_prompt,
        prompt,
        llama::image_bytes_to_url(image_bytes, mime_type),
        prev_messages,
        &model.port,
        id_slot,
    )
}

pub fn stop(model: &mut LlamaEmbedModel) -> Result<(), Box<dyn std::error::Error>> {
    model.program.kill()?;
    model.program.wait()?;
    Ok(())
}
