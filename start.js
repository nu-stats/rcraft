module.exports = {
  run: [
    {
      // Opens the RCraft window. The script ends when the window closes.
      method: "shell.run",
      params: {
        message: [
          "\"{{path.resolve(cwd, 'target', 'release', 'rcraft-desktop')}}\""
        ]
      }
    }
  ]
}
