'use strict'

import * as Electron  from 'electron'
import * as isDev     from 'electron-is-dev'
import * as minimist  from 'minimist'
import * as path      from 'path'
import * as pkg       from '../package.json'
import * as Server    from 'enso-studio-common/src/server'



// ================
// === Defaults ===
// ================

let windowCfg = {
    width  : 1024,
    height : 768,
}



// =============
// === Utils ===
// =============

function kebabToCamelCase(str){
  let arr     = str.split('-');
  let capital = arr.map((item,index) => {
      return index ? item.charAt(0).toUpperCase() + item.slice(1).toLowerCase() : item
  })
  return capital.join("");
}

function parseCmdArgs() {
    let argv = isDev ? process.argv.slice(process.argv.indexOf('--') + 1) : process.argv
    let args = minimist(argv)
    for (let argName in args) {
        let newName = kebabToCamelCase(argName)
        args[newName] = args[argName]
    }
    return args
}



// ==================================
// === Command Line Args Handlers ===
// ==================================

const HELP_MESSAGE = `
${pkg.build.productName} ${pkg.version} command line interface.

Usage: ${pkg.build.productName} [options]

Options:
    --debug-scene [SCENE]  Run the debug scene instead of the main app.
    --dev                  Run the application in development mode.
    --devtron              Install the Devtron Developer Tools extension.
    --no-server            Do not run server, connect to existing one instead.
    --no-window            Do not show window. Run in a batch mode.
    --port                 Port to use [${Server.DEFAULT_PORT}].
    --help                 Print the help message and exit.
    --window-size [SIZE]   Set the window size [${windowCfg.width}x${windowCfg.height}].
    --version              Print the version and exit.
`

let args = parseCmdArgs()

if (args.help) {
    console.log(HELP_MESSAGE)
    process.exit()
}

if (args.version) {
    console.log(pkg.version)
    process.exit();
}

if (args.windowSize) {
    let size   = args.windowSize.split('x')
    let width  = parseInt(size[0])
    let height = parseInt(size[1])
    if (isNaN(width) || isNaN(height)) {
        console.error(`Incorrect window size provided '${args.windowSize}'.`)
    } else {
        windowCfg.width  = width
        windowCfg.height = height
    }
}



// ================
// === Security ===
// ================

// === WebView Security ===

/// A WebView created in a renderer process that does not have Node.js integration enabled will not
/// be able to enable integration itself. However, a WebView will always create an independent
/// renderer process with its own webPreferences. It is a good idea to control the creation of new
/// <webview> tags from the main process and to verify that their webPreferences do not disable
/// security features. Follow the link to learn more:
/// https://www.electronjs.org/docs/tutorial/security#11-verify-webview-options-before-creation
function secureWebPreferences(webPreferences) {
    if(!webPreferences) { webPreferences = {} }
    delete webPreferences.preload
    delete webPreferences.preloadURL
    delete webPreferences.nodeIntegration
    delete webPreferences.nodeIntegrationInWorker
    delete webPreferences.webSecurity
    delete webPreferences.allowRunningInsecureContent
    delete webPreferences.experimentalFeatures
    delete webPreferences.enableBlinkFeatures
    delete webPreferences.allowpopups
    webPreferences.contextIsolation   = true
    webPreferences.enableRemoteModule = false
    return webPreferences
}

let urlWhitelist = []
Electron.app.on('web-contents-created', (event,contents) => {
    contents.on('will-attach-webview', (event,webPreferences,params) => {
        secureWebPreferences(webPreferences)
        if (!urlWhitelist.includes(params.src)) {
            console.error(`Blocked the creation of WebView pointing to '${params.src}'`)
            event.preventDefault()
        }
    })
})


// === Prevent Navigation ===

/// Navigation is a common attack vector. If an attacker can convince your app to navigate away from
/// its current page, they can possibly force your app to open web sites on the Internet. Follow the
/// link to learn more:
/// https://www.electronjs.org/docs/tutorial/security#12-disable-or-limit-navigation
Electron.app.on('web-contents-created', (event,contents) => {
    contents.on('will-navigate', (event,navigationUrl) => {
        event.preventDefault()
        console.error(`Prevented navigation to '${navigationUrl}'`)
    })
})


// === Disable New Windows Creation ===

/// Much like navigation, the creation of new webContents is a common attack vector. Attackers
/// attempt to convince your app to create new windows, frames, or other renderer processes with
/// more privileges than they had before or with pages opened that they couldn't open before.
/// Follow the link to learn more:
/// https://www.electronjs.org/docs/tutorial/security#13-disable-or-limit-creation-of-new-windows
Electron.app.on('web-contents-created', (event,contents) => {
    contents.on('new-window', async (event,navigationUrl) => {
        event.preventDefault()
        console.error(`Blocking new window creation request to '${navigationUrl}'`)
    })
})



// ============
// === Main ===
// ============

let hideInsteadOfQuit = false


let server     = null
let mainWindow = null

async function main() {
    if(args.server !== false) {
        let serverCfg      = Object.assign({},args)
        serverCfg.dir      = Electron.app.getAppPath()
        serverCfg.fallback = '/assets/index.html'
        server             = await Server.create(serverCfg)
    }
    mainWindow         = createWindow()
    mainWindow.on("close", (evt) => {
       if (hideInsteadOfQuit) {
           evt.preventDefault()
           mainWindow.hide()
       }
   })
}

let port = Server.DEFAULT_PORT
if      (server)    { port = server.port }
else if (args.port) { port = args.port }


function urlParamsFromObject(obj) {
    let params = []
    for (let key in obj) {
        let val = obj[key]
        if      (val === false) {}
        else if (val === true)  { params.push(key) }
        else                    { params.push(`${key}=${val}`) }
    }
    return params.join("&")
}

function createWindow() {

    let preferences = secureWebPreferences()
    if (args.devtron) {
        preferences.preload = path.join(Electron.app.getAppPath(),'..','assets','preload.js')
        preferences.enableRemoteModule = true
    }

    const window = new Electron.BrowserWindow({
        webPreferences : preferences,
        width          : windowCfg.width,
        height         : windowCfg.height,
        frame          : false,
        transparent    : true
    })

    if (args.dev) {
        window.webContents.openDevTools()
    }

    let cfg = {
        desktop      : true,
        dark         : Electron.nativeTheme.shouldUseDarkColors,
        highContrast : Electron.nativeTheme.shouldUseHighContrastColors,
    }

    let params      = urlParamsFromObject(cfg)
    let targetScene = ""
    if (args.debugScene) {
        targetScene = `debug/${args.debugScene}`
    }
    let address = `http://localhost:${port}/${targetScene}?${params}`
    console.log(`Loading the window address ${address}`)
    window.loadURL(address)

    return window
}

/// By default, Electron will automatically approve all permission requests unless the developer has
/// manually configured a custom handler. While a solid default, security-conscious developers might
/// want to assume the very opposite. Follow the link to learn more:
// https://www.electronjs.org/docs/tutorial/security#4-handle-session-permission-requests-from-remote-content
function setupPermissions() {
    Electron.session.defaultSession.setPermissionRequestHandler (
        (webContents,permission,callback) => {
            const url = webContents.getURL()
            console.error(`Unhandled permission request '${permission}'.`)
        }
    )
}



// ==============
// === Events ===
// ==============

Electron.app.on('activate', () => {
    if (process.platform == 'darwin') {
        mainWindow.show()
    }
})

Electron.app.on('ready', () => {
    if(args.window !== false) {
        main()
    }
})

if (process.platform === 'darwin') {
    hideInsteadOfQuit = true
    Electron.app.on('before-quit', function() {
        hideInsteadOfQuit = false
    })
}



// =============================
// === Deprecations & Fixmes ===
// =============================

/// FIXME: Will not be needed in Electron 9 anymore.
Electron.app.allowRendererProcessReuse = true

// FIXME Enable Metal backend on MacOS https://github.com/electron/electron/issues/22465
