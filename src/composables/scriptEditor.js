import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { listen } from '@tauri-apps/api/event'

let editorWindow = null

export async function openScriptEditor(scriptId = null) {
  const label = 'script-editor'
  const url = scriptId
    ? `script-editor.html?id=${encodeURIComponent(scriptId)}`
    : 'script-editor.html'

  const existing = await WebviewWindow.getByLabel(label)
  if (existing) {
    await existing.setFocus()
    if (scriptId) {
      await existing.emit('load-script', { scriptId })
    }
    return existing
  }

  editorWindow = new WebviewWindow(label, {
    url,
    title: scriptId ? '编辑脚本' : '新建脚本',
    width: 900,
    height: 700,
    minWidth: 600,
    minHeight: 400,
    resizable: true,
    center: true,
  })

  return editorWindow
}

export function onScriptSaved(callback) {
  return listen('script-saved', callback)
}

export function onScriptsChanged(callback) {
  return listen('scripts-changed', callback)
}
