module.exports = {
  run: [
    // RCraft is written in Rust. Pinokio does not ship Rust, so rustup
    // installs a private toolchain in .rust/ (your own Rust, if any, is not
    // touched). On Windows the MSVC toolchain also needs the Visual Studio
    // C++ build tools.
    {
      when: "{{platform === 'win32'}}",
      method: "shell.run",
      params: {
        env: {
          CARGO_HOME: "{{path.resolve(cwd, '.rust', 'cargo')}}",
          RUSTUP_HOME: "{{path.resolve(cwd, '.rust', 'rustup')}}"
        },
        message: [
          "curl -fsSL --create-dirs -o .rust/rustup-init.exe https://static.rust-lang.org/rustup/dist/{{arch === 'arm64' ? 'aarch64' : 'x86_64'}}-pc-windows-msvc/rustup-init.exe",
          ".rust\\rustup-init.exe -y --no-modify-path --profile minimal --default-toolchain stable"
        ]
      }
    },
    {
      when: "{{platform !== 'win32'}}",
      method: "shell.run",
      params: {
        env: {
          CARGO_HOME: "{{path.resolve(cwd, '.rust', 'cargo')}}",
          RUSTUP_HOME: "{{path.resolve(cwd, '.rust', 'rustup')}}"
        },
        message: [
          "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable"
        ]
      }
    },
    {
      // Builds the desktop window and the command-line console into
      // target/release (several minutes the first time).
      method: "shell.run",
      params: {
        env: {
          CARGO_HOME: "{{path.resolve(cwd, '.rust', 'cargo')}}",
          RUSTUP_HOME: "{{path.resolve(cwd, '.rust', 'rustup')}}"
        },
        message: [
          "\"{{path.resolve(cwd, '.rust', 'cargo', 'bin', 'cargo')}}\" build --release -p rcraft-desktop -p rcraft-cli"
        ]
      }
    },
    {
      method: "notify",
      params: {
        html: "Click the 'Start' tab to get started!"
      }
    }
  ]
}
