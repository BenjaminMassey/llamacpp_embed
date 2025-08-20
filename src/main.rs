use std::process::{Command, Stdio};
use std::io::{BufReader, Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;

const LLAMA_CLI: &str = r#"C:\Users\benjamin.massey\Downloads\llama-b6205-bin-win-cuda-12.4-x64\llama-cli.exe"#;
const GGUF_MODEL: &str = r#"C:\Users\benjamin.massey\Downloads\Qwen3-14B-UD-IQ2_M.gguf"#;

fn main() {
    println!("Loading model...");
    let mut model = start(GGUF_MODEL);
    println!("Model loaded.\n");
    for prompt in vec![
        "what year did the US declare independence? reply only with the year. /no_think",
        "who was the first president of the US? reply only with the name. /no_think",
        "write me a piece of Rust code which takes in a command line argument of number of characters and prints a string that contains random letters of the given number of characters /no_think",
    ] {
        println!("Prompting with \"{}\"...", &prompt);
        println!("Response: {}\n", &chat(&mut model, &prompt));
    }
    println!("Stopping model...");
    stop(&mut model);
    println!("Model stopped.");
}

pub struct LlamaEmbedModel {
    program: std::process::Child,
    input: std::process::ChildStdin,
    receiver: Receiver<u8>,
}

pub fn start(gguf_path: &str) -> LlamaEmbedModel {
    let mut child = Command::new(LLAMA_CLI)
        .args(&["-m", gguf_path])
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
                break; // EOF
            }
            // Ignore send errors if receiver is gone
            if tx.send(buffer[0]).is_err() {
                break;
            }
        }
    }); // TODO: do we need to kill this?

    // dummy message in order to wait for model loading text plus flush the chat
    send_command(&mut stdin, "respond only with the text 'hello'. /no_think");
    let _ = read_response(&rx, std::time::Duration::from_secs(2), false);

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
                if cli_break && buffer.len() > 4 &&
                    buffer[buffer.len() - 1] == 32 && // space
                    buffer[buffer.len() - 2] == 62 && // >
                    buffer[buffer.len() - 3] == 10 && // LF
                    buffer[buffer.len() - 4] == 13 // CR
                {
                    break
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !cli_break && buffer.len() > 4 {
                    break
                }
            },
            Err(_) => {
                if buffer.len() > 4 {
                    break
                }
            },
        }
    }
    remove_think_tag(String::from_utf8_lossy(&buffer[..buffer.len() - 4]).trim())
}

fn remove_think_tag(text: &str) -> String {
    let tag = "</think>";
    match text.find(tag) {
        Some(index) => text[(index + tag.len())..].trim().to_string(),
        None => text.to_string(),
    }
}