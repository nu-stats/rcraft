module.exports = {
  run: [
    {
      // The RCraft console (`rcraft-cli` with no arguments): type R code and
      // it runs in R, as the window's Command line does. Type `:quit` to leave.
      // `rcraft-cli --find-r` lists the R installations RCraft can see.
      method: "shell.run",
      params: {
        input: true,
        message: [
          "\"{{path.resolve(cwd, 'target', 'release', 'rcraft-cli')}}\""
        ]
      }
    }
  ]
}
