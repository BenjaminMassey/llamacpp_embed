mod llama;

pub struct LlamaEmbedModel {
    program: std::process::Child,
    system_prompt: String,
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
    system_prompt: &str,
    load_timeout: u64,
) -> Result<LlamaEmbedModel, Box<dyn std::error::Error>> {
    if !std::path::Path::new(gguf_path).exists() {
        return Err(format!("Model not found: \"{}\".", gguf_path).into());
    }

    let log = std::fs::File::create("llamacpp_log.txt").unwrap();
    let program =
        std::process::Command::new(&std::path::Path::new(&llama_cli_path()).to_str().unwrap())
            .args(&["-m", gguf_path, "--port", "8080"])
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
    })
}

pub fn chat(
    model: &mut LlamaEmbedModel,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    llama::chat(&model.system_prompt, prompt)
}

pub fn stop(model: &mut LlamaEmbedModel) -> Result<(), Box<dyn std::error::Error>> {
    model.program.kill()?;
    model.program.wait()?;
    Ok(())
}
