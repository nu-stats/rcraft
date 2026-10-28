module.exports = {
  run: [{
    method: "shell.run",
    params: {
      message: "git pull"
    }
  }, {
    // After pulling, bring the private Rust toolchain up to date and rebuild.
    method: "shell.run",
    params: {
      env: {
        CARGO_HOME: "{{path.resolve(cwd, '.rust', 'cargo')}}",
        RUSTUP_HOME: "{{path.resolve(cwd, '.rust', 'rustup')}}"
      },
      message: [
        "\"{{path.resolve(cwd, '.rust', 'cargo', 'bin', 'rustup')}}\" update stable --no-self-update",
        "\"{{path.resolve(cwd, '.rust', 'cargo', 'bin', 'cargo')}}\" build --release -p rcraft-desktop -p rcraft-cli"
      ]
    }
  }]
}
