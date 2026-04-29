use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let output_dir = get_output_dir();
    if output_dir.exists() {
        return;
    }
    let url = get_release_url().unwrap();
    get_llama_cpp(&url, &output_dir);
    make_model_folder();
    copy_deploy_scripts();
    inject_gitignore();
    println!("cargo:rerun-if-changed=*");
}

fn get_output_dir() -> std::path::PathBuf {
    let dir_name = "llama-cpp";
    let target_dir = get_project_directory();
    let target_path = Path::new(&target_dir);
    target_path.join(&dir_name)
}

#[cfg(target_os = "windows")]
fn get_release_url() -> Result<String, Box<dyn std::error::Error>> {
    release_url(&vec![
        "win".to_owned(),
        "vulkan".to_owned(),
        "x64".to_owned(),
    ])
}
#[cfg(not(target_os = "windows"))]
fn get_release_url() -> Result<String, Box<dyn std::error::Error>> {
    release_url(&vec![
        "ubuntu".to_owned(),
        "vulkan".to_owned(),
        "x64".to_owned(),
    ])
}
// TODO: should get user preference for backend and build system from cargo, rather than hard-set

#[derive(Debug, serde::Deserialize)]
struct Release {
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

fn release_url(parameters: &[String]) -> Result<String, Box<dyn std::error::Error>> {
    let release = reqwest::blocking::Client::new()
        .get("https://api.github.com/repos/ggml-org/llama.cpp/releases/latest")
        .header("User-Agent", "llamacpp_embed")
        .send()?
        .error_for_status()?
        .json::<Release>()?;

    let mut matched_release: Option<String> = None;
    for asset in &release.assets {
        println!("{}", asset.name);
        let mut missing_some_parameter = false;
        for paramater in parameters {
            if !asset.name.contains(paramater) {
                missing_some_parameter = true;
                break;
            }
        }
        if missing_some_parameter {
            continue;
        }
        matched_release = Some(asset.browser_download_url.clone());
        break;
    }

    match matched_release {
        Some(url) => return Ok(url),
        None => return Err("Failed to find proper GitHub release.".to_owned().into()),
    }
}

fn get_llama_cpp(url: &str, output_dir: &std::path::PathBuf) {
    if url.ends_with(".tar.gz") {
        std::fs::create_dir_all(&output_dir).unwrap();
        let output_tar = output_dir.join("llama-cpp.tar.gz");
        download_file(&url, &output_tar);
        let file = std::fs::File::open(&output_tar).unwrap();
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(&output_dir).unwrap();
        std::fs::remove_file(&output_tar).unwrap();
    } else if url.ends_with(".zip") {
        std::fs::create_dir_all(&output_dir).unwrap();
        let output_zip = output_dir.join("llama-cpp.zip");
        download_file(&url, &output_zip);
        let file = std::fs::File::open(&output_zip).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        archive.extract(&output_dir).unwrap();
        std::fs::remove_file(&output_zip).unwrap();
    } else {
        panic!("Unknown file type from URL: {}", url);
    }
    rework_subfolder_if_needed(&output_dir);
}

fn get_project_directory() -> String {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
} // TODO: better

fn download_file(url: &str, output_path: &std::path::PathBuf) {
    if output_path.exists() {
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

fn rework_subfolder_if_needed(output_dir: &Path) {
    if output_dir.join("llama-server").exists() {
        return;
    }

    let entries: Vec<_> = std::fs::read_dir(output_dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();

    if entries.len() == 1 && entries[0].file_type().unwrap().is_dir() {
        let inner_dir = entries[0].path();

        for entry in std::fs::read_dir(&inner_dir).unwrap() {
            let entry = entry.unwrap();
            let dest = output_dir.join(entry.file_name());
            std::fs::rename(entry.path(), dest).unwrap();
        }

        std::fs::remove_dir(&inner_dir).unwrap();
    }
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
    let desired_ignores = vec!["/llama-model/*.gguf", "/llama-cpp", "/deployments"];
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
