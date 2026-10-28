module.exports = {
  // Removes what install.js added (the private Rust toolchain and the build).
  // The source code is kept.
  run: [{
    method: "fs.rm",
    params: {
      path: ".rust"
    }
  }, {
    method: "fs.rm",
    params: {
      path: "target"
    }
  }]
}
