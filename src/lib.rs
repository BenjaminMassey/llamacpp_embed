use std::process::{Command, Stdio};
use std::io::{BufReader, Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub struct LlamaEmbedModel {
    program: std::process::Child,
    input: std::process::ChildStdin,
    receiver: Receiver<u8>,
}

#[cfg(target_os = "windows")]
fn llama_cli_path() -> String {
    "./llama-windows/llama-cli.exe".to_owned()
}
#[cfg(not(target_os = "windows"))]
fn llama_cli_path() -> String {
    "./llama-linux/build/bin/llama-cli".to_owned()
}

pub fn start(gguf_path: &str, system_prompt: &str) -> LlamaEmbedModel {
    if !std::path::Path::new(gguf_path).exists() {
        panic!("Model not found: \"{}\".", gguf_path);
    }
    
    #[cfg(target_os = "windows")]
    let mut child = Command::new(&std::path::Path::new(&llama_cli_path()).to_str().unwrap())
        .args(&["-m", gguf_path, "-sys", system_prompt])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new(&std::path::Path::new(&llama_cli_path()).to_str().unwrap())
        .args(&["-m", gguf_path, "-sys", system_prompt, "-i", "--simple-io"])
        .env("TERM", "dumb")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    
    let (tx, rx) = mpsc::channel::<u8>();
    let _ = thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buffer = [0u8; 1];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            if tx.send(buffer[0]).is_err() {
                break;
            }
        }
    }); // TODO: do we need to kill this?
    
    // wait for model loading text
    let _ = read_response(&rx);
	
    LlamaEmbedModel {
        program: child,
        input: stdin,
        receiver: rx
    }
}

pub fn chat(model: &mut LlamaEmbedModel, prompt: &str) -> String {
    send_command(&mut model.input, prompt);
    read_response(&model.receiver)
}

pub fn stop(model: &mut LlamaEmbedModel) {
    model.program.kill().unwrap();
    model.program.wait().unwrap();
}

fn send_command(stdin: &mut std::process::ChildStdin, command: &str) {
    writeln!(stdin, "{}", command).unwrap();
    stdin.flush().unwrap();
}

fn read_response(rx: &mpsc::Receiver<u8>) -> String {
    let timeout = std::time::Duration::from_secs(10);
    #[cfg(target_os = "windows")]
    let target_count = 4; // windows has CR
    #[cfg(not(target_os = "windows"))]
    let target_count = 3; // no CR on linux
    let mut buffer = Vec::new();
    loop {
        match rx.recv_timeout(timeout) {
            Ok(byte) => {
                buffer.push(byte);
                if buffer.len() >= target_count {
                    #[cfg(target_os = "windows")]
                    let extra = buffer[buffer.len() - 4] == 13; // CR
                    #[cfg(not(target_os = "windows"))]
                    let extra = true; // no CR on linux
                    if buffer[buffer.len() - 1] == 32 && // space
                        buffer[buffer.len() - 2] == 62 && // >
                        buffer[buffer.len() - 3] == 10 && // LF
                        extra 
                    {
                        break
                    }
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(_) => break,
        }
    }
    if buffer.len() >= target_count {
        return remove_think_tag(String::from_utf8_lossy(&buffer[..buffer.len() - target_count]).trim());
    }
    "!FAILURE!".to_owned() // TODO: better, probably -> Option<String> or similar
}

fn remove_think_tag(text: &str) -> String {
    let tag = "</think>";
    match text.find(tag) {
        Some(index) => text[(index + tag.len())..].trim().to_string(),
        None => text.to_string(),
    }
}
