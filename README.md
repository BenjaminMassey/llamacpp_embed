# llamacpp_embed
Copyright &copy; 2025 Benjamin Massey (Version 0.1.0)

`llamacpp_embed`: a library for bundling llama.cpp runtime into rust projects

## Example

### Add to `[dependencies]` in `Cargo.toml`:

```toml
llamacpp_embed = { git = "https://www.github.com/BenjaminMassey/llamacpp_embed" }
```

### `src/main.rs`:

```rust
fn main() {
    let mut model = llamacpp_embed::start("./llama-model/model.gguf");
    let prompt = "How can I write \"Hello, World!\" in Rust?";
    println!(
        "{}\n\n=>\n\n{}",
        prompt,
        &llamacpp_embed::chat(&mut model, prompt),
    );
    llamacpp_embed::stop(&mut model);
}
```

## Additional Notes

The first `cargo build` for your project may take quite a while: `llamacpp_embed` will download runnable binaries into your project. It will place the llama.cpp runtime in a folder `llama-windows` or `llama-linux`, depending on your system. It will also create a folder `llama-model` where you can place your GGUF model file, which will be used for deployments.

In order to package your program, you can run copied-in `deploy-win.bat` or `deploy-lin.sh` scripts. These will create a `deployments` folder in your project's folder, in which there will be subfolders for `windows` and `linux`. Builds will be their own folders within these, which will be named along the structure of `build_<DATE>_<TIME>`. Note that depending on specifics to your software, you may have additional steps to make sure your deployed version has access to all necessary resources: this is only covering `llamacpp_embed` files.