use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    get_llama_cpp();
    make_model_folder();
    copy_deploy_scripts();
    inject_gitignore();
    println!("cargo:rerun-if-changed=*");
}

fn download_file(url: &str, output: &str) {
    let target_dir = get_project_directory();
    let target_path = Path::new(&target_dir);
    let output_path = target_path.join(output);
    if Path::new(&output_path).exists() {
        return;
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .unwrap();
    let response = client.get(url).send().unwrap();
    let mut file = File::create(&output_path).unwrap();
    let content = response.bytes().unwrap();
    file.write_all(&content).unwrap();
}

fn get_project_directory() -> String {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    Path::new(&out_dir)
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .parent().unwrap()
        .to_str().unwrap()
        .to_owned()
}

#[cfg(target_os = "windows")]
fn get_llama_names() -> (String, String) {
    (
        "llama-windows".into(),
        "https://github.com/ggml-org/llama.cpp/releases/download/b6209/llama-b6209-bin-win-cuda-12.4-x64.zip".into(),
    )
}
#[cfg(not(target_os = "windows"))]
fn get_llama_names() -> (String, String) {
    (
        "llama-linux".into(),
        "https://github.com/ggml-org/llama.cpp/releases/download/b6209/llama-b6209-bin-ubuntu-vulkan-x64.zip".into(),
    )
}

fn get_llama_cpp() {
    let (dir_name, zip_url) = get_llama_names();
    let target_dir = get_project_directory();
    let target_path = Path::new(&target_dir);
    let output_dir = target_path.join(&dir_name);
    if output_dir.exists() {
        return;
    }
    download_file(&zip_url, "llama-cpp.zip");
    let output_zip = target_path.join("llama-cpp.zip");
    std::fs::create_dir_all(&output_dir).unwrap();
    let file = std::fs::File::open(&output_zip).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    archive.extract(output_dir).unwrap();
    std::fs::remove_file(&output_zip).unwrap();
}

fn make_model_folder() {
    let target_dir = get_project_directory();
    let target_path = Path::new(&target_dir);
    let output_dir = target_path.join("llama-model");
    if output_dir.exists() {
        return;
    }
    std::fs::create_dir_all(&output_dir).unwrap();
}

fn copy_deploy_scripts() {
    let target_dir = get_project_directory();
    let target_path = Path::new(&target_dir);
    let win_path = target_path.join("deploy-win.bat");
    if !win_path.exists() {
        std::fs::copy("deploy-win.bat", &win_path).unwrap();
    }
    let lin_path = target_path.join("deploy-lin.sh");
    if !lin_path.exists() {
        std::fs::copy("deploy-lin.sh", &lin_path).unwrap();
    }
}

fn inject_gitignore() {
    let desired_ignores = vec![
        "/llama-model/*.gguf",
        "/llama-windows",
        "/llama-linux",
        "/deployments",
    ];
    let target_dir = get_project_directory();
    let target_path = Path::new(&target_dir);
    let ignore_path = target_path.join(".gitignore");
    let ignore_text = std::fs::read_to_string(&ignore_path).unwrap();
    let mut ignore_file = std::fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(&ignore_path)
        .unwrap();
    for ignore in desired_ignores {
        if !ignore_text.contains(ignore) {
            let _ = write!(ignore_file, "\n{}", ignore);
        }
    }
}
