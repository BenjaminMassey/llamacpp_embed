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

pub fn start(gguf_path: &str) -> LlamaEmbedModel {
    if !std::path::Path::new(gguf_path).exists() {
        panic!("Model not found: \"{}\".", gguf_path);
    }
    
    #[cfg(target_os = "windows")]
    let mut child = Command::new(&std::path::Path::new(&llama_cli_path()).to_str().unwrap())
        .args(&["-m", gguf_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new(&std::path::Path::new(&llama_cli_path()).to_str().unwrap())
        .args(&["-m", gguf_path, "-i", "--simple-io"])
        .env("TERM", "dumb")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    
    let mut stdin = child.stdin.take().unwrap();
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
    
    // dummy message in order to wait for model loading text plus flush the chat
    send_command(&mut stdin, "respond only with the text 'hello'. /no_think");
    for _ in 0..2 { // TODO: ew, why?
        read_response(&rx, std::time::Duration::from_secs(2), false);
    }
	
    LlamaEmbedModel {
        program: child,
        input: stdin,
        receiver: rx
    }
}

pub fn chat(model: &mut LlamaEmbedModel, prompt: &str) -> String {
    send_command(&mut model.input, prompt);
    read_response(&model.receiver, std::time::Duration::from_secs(0), true)
}

pub fn stop(model: &mut LlamaEmbedModel) {
    model.program.kill().unwrap();
    model.program.wait().unwrap();
}

fn send_command(stdin: &mut std::process::ChildStdin, command: &str) {
    writeln!(stdin, "{}", command).unwrap();
    stdin.flush().unwrap();
}

fn read_response(rx: &mpsc::Receiver<u8>, idle_timeout: std::time::Duration, cli_break: bool) -> String {
    let mut buffer = Vec::new();
    loop {
        match rx.recv_timeout(idle_timeout) {
            Ok(byte) => {
                buffer.push(byte);
                if cli_break && buffer.len() >= 3 &&
                    buffer[buffer.len() - 1] == 32 && // space
                    buffer[buffer.len() - 2] == 62 && // >
                    buffer[buffer.len() - 3] == 10 // LF
                {
                    break
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !cli_break && buffer.len() >= 3 {
                    break
                }
            },
            Err(_) => {
                if buffer.len() >= 3 {
                    break
                }
            },
        }
    }
    remove_think_tag(String::from_utf8_lossy(&buffer[..buffer.len() - 3]).trim())
}

fn remove_think_tag(text: &str) -> String {
    let tag = "</think>";
    match text.find(tag) {
        Some(index) => text[(index + tag.len())..].trim().to_string(),
        None => text.to_string(),
    }
}
