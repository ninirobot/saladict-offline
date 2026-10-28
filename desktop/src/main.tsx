import { getCurrentWindow } from '@tauri-apps/api/window'
import { render } from 'preact'

import { api } from './api'
import './styles/app.css'
import { MainWindow } from './views/MainWindow'
import { Popup } from './views/Popup'

window.addEventListener('error', event => {
  void api.log(`${event.message} @ ${event.filename}:${event.lineno}`, 'error')
})
window.addEventListener('unhandledrejection', event => {
  void api.log(`unhandled rejection: ${String(event.reason)}`, 'error')
})

function currentLabel(): string {
  try {
    return getCurrentWindow().label
  } catch {
    return 'main'
  }
}

const root = document.getElementById('app')
if (root) {
  render(currentLabel() === 'popup' ? <Popup /> : <MainWindow />, root)
}
