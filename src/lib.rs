mod llama;

pub struct LlamaEmbedModel {
    program: std::process::Child,
    system_prompt: String,
    image_capable: bool,
}

#[cfg(target_os = "windows")]
fn llama_cli_path() -> String {
    "./llama-cpp/llama-server.exe".to_owned()
}
#[cfg(not(target_os = "windows"))]
fn llama_cli_path() -> String {
    "./llama-cpp/llama-server".to_owned()
}

pub fn start(
    gguf_path: &str,
    mmproj_path: Option<&str>,
    system_prompt: &str,
    load_timeout: u64,
) -> Result<LlamaEmbedModel, Box<dyn std::error::Error>> {
    if !std::path::Path::new(gguf_path).exists() {
        return Err(format!("Model not found: \"{}\".", gguf_path).into());
    }

    let args = if let Some(mmproj) = mmproj_path {
        vec!["-m", gguf_path, "--mmproj", mmproj, "--port", "8080"]
    } else {
        vec!["-m", gguf_path, "--port", "8080"]
    };
    let log = std::fs::File::create("llamacpp_log.txt").unwrap();
    let program =
        std::process::Command::new(&std::path::Path::new(&llama_cli_path()).to_str().unwrap())
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
    })
}

pub fn chat(
    model: &mut LlamaEmbedModel,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    llama::chat(&model.system_prompt, prompt)
}

pub fn chat_with_image(
    model: &mut LlamaEmbedModel,
    prompt: &str,
    image_path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    if !model.image_capable {
        return Err("llamacpp_embed::start(..) was not provided with an MMPROJ file.".into());
    }
    llama::chat_with_image(&model.system_prompt, prompt, image_path)
}
pub fn stop(model: &mut LlamaEmbedModel) -> Result<(), Box<dyn std::error::Error>> {
    model.program.kill()?;
    model.program.wait()?;
    Ok(())
}
