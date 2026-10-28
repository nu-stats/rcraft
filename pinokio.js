const path = require('path')
module.exports = {
  version: "7.0",
  title: "RCraft",
  description: "A point-and-click front end for R: menus, dialogs, a command window, data grid, graphs and a package finder. Every action shows the R code it runs. Needs R installed on this computer.",
  menu: async (kernel, info) => {
    let exe = kernel.platform === "win32" ? ".exe" : ""
    let installed = info.exists(`target/release/rcraft-desktop${exe}`)
    let running = {
      install: info.running("install.js"),
      start: info.running("start.js"),
      console: info.running("console.js"),
      update: info.running("update.js"),
      reset: info.running("reset.js")
    }
    if (running.install) {
      return [{
        default: true,
        icon: "fa-solid fa-plug",
        text: "Installing",
        href: "install.js",
      }]
    } else if (installed) {
      if (running.start) {
        return [{
          default: true,
          icon: 'fa-solid fa-terminal',
          text: "Running",
          href: "start.js",
        }]
      } else if (running.console) {
        return [{
          default: true,
          icon: 'fa-solid fa-terminal',
          text: "Console",
          href: "console.js",
        }]
      } else if (running.update) {
        return [{
          default: true,
          icon: 'fa-solid fa-terminal',
          text: "Updating",
          href: "update.js",
        }]
      } else if (running.reset) {
        return [{
          default: true,
          icon: 'fa-solid fa-terminal',
          text: "Resetting",
          href: "reset.js",
        }]
      } else {
        return [{
          default: true,
          icon: "fa-solid fa-power-off",
          text: "Start",
          href: "start.js",
        }, {
          icon: "fa-solid fa-terminal",
          text: "Console",
          href: "console.js",
        }, {
          icon: "fa-solid fa-plug",
          text: "Update",
          href: "update.js",
        }, {
          icon: "fa-solid fa-plug",
          text: "Install",
          href: "install.js",
        }, {
          icon: "fa-regular fa-circle-xmark",
          text: "Reset",
          href: "reset.js",
        }]
      }
    } else {
      return [{
        default: true,
        icon: "fa-solid fa-plug",
        text: "Install",
        href: "install.js",
      }]
    }
  }
}
