import { defineStore } from 'pinia'
import { QUICK_RULES } from '../constants/quickRules'

async function getInvoke() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke
  } catch (error) {
    console.error('Failed to import invoke:', error)
    throw new Error('Tauri API 不可用，请在 Tauri 应用中运行')
  }
}

async function getOpen() {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    return open
  } catch (error) {
    console.error('Failed to import open:', error)
    throw new Error('Tauri API 不可用，请在 Tauri 应用中运行')
  }
}

async function loadSavedScripts() {
  try {
    const invoke = await getInvoke()
    return await invoke('load_saved_scripts')
  } catch (error) {
    console.error('Failed to load saved scripts:', error)
    return []
  }
}

const MD5_PREVIEW_PLACEHOLDER = '[md5-preview]'

function isPlaceholderPreviewName(name) {
  return name.includes(MD5_PREVIEW_PLACEHOLDER)
}

function buildPreviewFingerprint(state) {
  const params = state.currentScriptId
    ? state.scriptParams[state.currentScriptId] || {}
    : {}
  return JSON.stringify({
    files: state.files.map((f) => f.path),
    basePath: state.basePath,
    script: state.script,
    currentScriptId: state.currentScriptId,
    previewMd5Mode: state.previewMd5Mode,
    params,
  })
}

const PREVIEW_MD5_MODE_KEY = 'previewMd5Mode'

function loadPreviewMd5Mode() {
  try {
    const v = localStorage.getItem(PREVIEW_MD5_MODE_KEY)
    if (v === 'always' || v === 'skip' || v === 'auto') return v
  } catch {
    // ignore
  }
  return 'auto'
}

export const useRenameStore = defineStore('rename', {
  state: () => ({
    files: [],
    script: `function rename() {
  return {[__filePath]: __fileName};
}`,
    previewResults: [],
    hasPreviewed: false,
    previewFingerprint: null,
    md5PreviewPlaceholder: false,
    previewMd5Mode: loadPreviewMd5Mode(),
    previewInProgress: false,
    previewProgress: { completed: 0, total: 0, currentPath: null, phase: '' },
    previewCancelled: false,
    executeInProgress: false,
    executeProgress: { completed: 0, total: 0, currentPath: null, phase: '' },
    sessionScriptLogs: [],
    history: [],
    undoStack: [],
    basePath: null,
    folderFileCount: null,
    loading: false,
    error: null,
    savedScripts: [],
    currentScriptId: null,
    scriptParams: {},
    scriptLogs: [],
    showHistoryDrawer: false,
  }),

  getters: {
    hasFiles: (state) => state.files.length > 0,
    hasPreview: (state) => state.previewResults.length > 0,
    canUndo: (state) => state.undoStack.length > 0,
    scriptValid: (state) => state.script.trim().length > 0,
    previewStale: (state) => {
      if (!state.hasPreviewed || !state.previewFingerprint) return false
      return buildPreviewFingerprint(state) !== state.previewFingerprint
    },
    changedPreviewCount: (state) =>
      state.previewResults.filter(
        (r) => r.original !== r.new_name && !r.error
      ).length,
    conflictPaths: (state) => {
      const counts = {}
      for (const r of state.previewResults) {
        if (r.error || r.original === r.new_name) continue
        if (isPlaceholderPreviewName(r.new_name)) continue
        counts[r.new_name] = (counts[r.new_name] || 0) + 1
      }
      return Object.keys(counts).filter((k) => counts[k] > 1)
    },
    conflictDetails: (state) => {
      const mapping = {}
      for (const r of state.previewResults) {
        if (r.error || r.original === r.new_name) continue
        if (isPlaceholderPreviewName(r.new_name)) continue
        if (!mapping[r.new_name]) mapping[r.new_name] = []
        mapping[r.new_name].push(r.original)
      }
      return Object.entries(mapping)
        .filter(([, originals]) => originals.length > 1)
        .map(([newName, originals]) => ({ newName, originals }))
    },
    hasConflicts() {
      return this.conflictPaths.length > 0
    },
    workflowStep: (state) => {
      if (!state.hasFiles) return 1
      if (!state.scriptValid) return 2
      if (!state.hasPreviewed || buildPreviewFingerprint(state) !== state.previewFingerprint) return 3
      return 4
    },
  },

  actions: {
    invalidatePreview() {
      this.previewResults = []
      this.hasPreviewed = false
      this.previewFingerprint = null
      this.md5PreviewPlaceholder = false
      this.previewInProgress = false
      this.previewProgress = { completed: 0, total: 0, currentPath: null, phase: '' }
      this.previewCancelled = false
      this.sessionScriptLogs = []
    },

    async initSavedScripts() {
      this.savedScripts = await loadSavedScripts()
    },

    async loadScriptLogs(date = null) {
      try {
        const invoke = await getInvoke()
        this.scriptLogs = await invoke('load_script_logs', { date })
      } catch (error) {
        console.error('Failed to load script logs:', error)
        this.scriptLogs = []
      }
    },

    async initScriptLogs() {
      await this.loadScriptLogs()
    },

    async initHistory() {
      try {
        const invoke = await getInvoke()
        const history = await invoke('load_history')
        this.history = history || []
      } catch (error) {
        console.error('Failed to load history:', error)
      }
    },

    async persistHistory() {
      try {
        const invoke = await getInvoke()
        await invoke('save_history', { history: this.history.slice(0, 50) })
      } catch (error) {
        console.error('Failed to save history:', error)
      }
    },

    async refreshFolderFileCount() {
      if (!this.basePath) {
        this.folderFileCount = null
        return
      }
      try {
        const invoke = await getInvoke()
        this.folderFileCount = await invoke('count_folder_files_recursive', {
          folderPath: this.basePath,
        })
      } catch (error) {
        console.error('Failed to count folder files:', error)
        this.folderFileCount = null
      }
    },

    async selectFiles() {
      try {
        const open = await getOpen()
        const selected = await open({ multiple: true, directory: false })

        if (selected) {
          const files = Array.isArray(selected) ? selected : [selected]
          const fileList = files.map((path) => ({
            name: path.split(/[/\\]/).pop(),
            path,
            size: 0,
            modified: new Date().toISOString(),
          }))
          this.files = [...this.files, ...fileList]
          this.basePath = null
          this.folderFileCount = null
          this.invalidatePreview()
        }
      } catch (error) {
        this.error = `选择文件失败: ${error}`
        console.error(error)
      }
    },

    async selectFolder() {
      try {
        const open = await getOpen()
        const selected = await open({ multiple: false, directory: true })

        if (selected) {
          this.basePath = selected
          const folderName = selected.split(/[/\\]/).pop() || selected
          this.files = [{
            name: folderName,
            path: folderName,
            size: 0,
            modified: new Date().toISOString(),
          }]
          this.invalidatePreview()
          await this.refreshFolderFileCount()
        }
      } catch (error) {
        this.error = `选择文件夹失败: ${error}`
        console.error(error)
      }
    },

    async addDroppedPaths(paths) {
      if (!paths || paths.length === 0) return

      this.error = null
      this.invalidatePreview()

      try {
        const { stat } = await import('@tauri-apps/plugin-fs')
        const dirs = []
        const files = []

        for (const p of paths) {
          const meta = await stat(p)
          if (meta.isDirectory) {
            dirs.push(p)
          } else {
            files.push(p)
          }
        }

        if (dirs.length > 0 && files.length > 0) {
          this.error = '请分别拖放文件或文件夹，不要混合拖放'
          return
        }

        if (dirs.length > 1) {
          this.error = '一次只能拖放一个文件夹'
          return
        }

        if (dirs.length === 1) {
          const selected = dirs[0]
          this.basePath = selected
          const folderName = selected.split(/[/\\]/).pop() || selected
          this.files = [{
            name: folderName,
            path: folderName,
            size: 0,
            modified: new Date().toISOString(),
          }]
          await this.refreshFolderFileCount()
          return
        }

        const fileList = files.map((path) => ({
          name: path.split(/[/\\]/).pop(),
          path,
          size: 0,
          modified: new Date().toISOString(),
        }))

        if (this.basePath) {
          this.basePath = null
          this.folderFileCount = null
          this.files = fileList
        } else if (this.hasFiles) {
          this.files = [...this.files, ...fileList]
        } else {
          this.files = fileList
        }
      } catch (error) {
        this.error = `拖放失败: ${error}`
        console.error(error)
      }
    },

    removeFile(index) {
      this.files.splice(index, 1)
      if (this.files.length === 0) {
        this.basePath = null
        this.folderFileCount = null
      }
      this.invalidatePreview()
    },

    clearFiles() {
      this.files = []
      this.basePath = null
      this.folderFileCount = null
      this.invalidatePreview()
    },

    updateScript(script) {
      this.script = script
      this.invalidatePreview()
      this.scriptLogs = []
      this.currentScriptId = null
    },

    loadQuickRule(ruleId) {
      const rule = QUICK_RULES.find((r) => r.id === ruleId)
      if (!rule) return false

      this.script = rule.script
      this.currentScriptId = ruleId
      this.invalidatePreview()

      if (rule.parameters) {
        if (!this.scriptParams[ruleId]) {
          this.scriptParams[ruleId] = {}
        }
        rule.parameters.forEach((param) => {
          if (this.scriptParams[ruleId][param.name] === undefined) {
            if (param.default !== undefined && param.default !== null) {
              this.scriptParams[ruleId][param.name] = param.default
            } else if (param.type === 'number') {
              this.scriptParams[ruleId][param.name] = 0
            } else if (param.type === 'boolean') {
              this.scriptParams[ruleId][param.name] = false
            } else {
              this.scriptParams[ruleId][param.name] = ''
            }
          }
        })
      }
      return true
    },

    getCurrentScriptParameters() {
      if (!this.currentScriptId) return []
      const saved = this.savedScripts.find((s) => s.id === this.currentScriptId)
      if (saved?.parameters?.length) return saved.parameters
      const quick = QUICK_RULES.find((r) => r.id === this.currentScriptId)
      return quick?.parameters || []
    },

    async previewRename() {
      if (!this.hasFiles || !this.scriptValid) {
        this.error = '请先选择文件并编写脚本'
        return
      }

      if (this.previewInProgress) {
        return
      }

      this.loading = true
      this.previewInProgress = true
      this.previewCancelled = false
      this.error = null
      this.previewResults = []
      this.sessionScriptLogs = []
      this.previewProgress = { completed: 0, total: 0, currentPath: null, phase: '' }
      this.hasPreviewed = false
      this.md5PreviewPlaceholder = false

      const { listen } = await import('@tauri-apps/api/event')
      const unlisteners = []

      const onChunk = (event) => {
        const { mappings = [], logs = [] } = event.payload || {}
        console.debug('[preview] chunk', mappings.length, 'mappings')
        for (const [original, new_name] of mappings) {
          this.previewResults.push({ original, new_name, error: null })
          if (new_name.includes(MD5_PREVIEW_PLACEHOLDER)) {
            this.md5PreviewPlaceholder = true
          }
        }
        if (logs.length > 0) {
          this.sessionScriptLogs.push(...logs)
        }
      }

      const onProgress = (event) => {
        const p = event.payload || {}
        const completed = p.completed ?? 0
        const total = p.total ?? 0
        const currentPath = p.currentPath ?? p.current_path ?? null
        const phase = p.phase ?? ''
        console.debug('[preview] progress', completed, '/', total, phase, currentPath)
        this.previewProgress = { completed, total, currentPath, phase }
      }

      let completeResolver = () => {}
      const completePromise = new Promise((resolve) => {
        completeResolver = resolve
      })

      const onComplete = (event) => {
        const payload = event.payload || {}
        console.debug('[preview] complete', payload)
        this.hasPreviewed = true
        this.previewFingerprint = buildPreviewFingerprint(this)
        this.md5PreviewPlaceholder =
          payload.md5PreviewPlaceholder ??
          payload.md5_preview_placeholder ??
          this.md5PreviewPlaceholder
        this.previewCancelled = !!payload.cancelled

        const errors = payload.errors || []
        if (errors.length > 0) {
          this.error = errors.join('\n')
        } else if (payload.cancelled) {
          this.error = this.previewResults.length > 0
            ? '预览已取消（已保留已完成的结果）'
            : '预览已取消'
        } else {
          this.error = null
        }
        completeResolver()
      }

      try {
        unlisteners.push(await listen('preview-chunk', onChunk))
        unlisteners.push(await listen('preview-progress', onProgress))
        unlisteners.push(await listen('preview-complete', onComplete))

        const invoke = await getInvoke()
        const fileNames = this.basePath
          ? [this.basePath]
          : this.files.map((f) => f.path)
        const params = this.getCurrentScriptParams()

        let invokeError = null
        const invokePromise = invoke('preview_rename', {
          files: fileNames,
          script: this.script,
          basePath: null,
          params: params && Object.keys(params).length > 0 ? params : null,
          previewMd5Mode: this.previewMd5Mode,
        }).catch((e) => {
          invokeError = e
          completeResolver()
          return null
        })

        await completePromise
        await invokePromise
        if (invokeError) {
          throw invokeError
        }
      } catch (error) {
        completeResolver()
        this.error = `预览失败: ${error}`
        console.error(error)
        if (!this.hasPreviewed) {
          this.previewResults = []
        }
      } finally {
        unlisteners.forEach((fn) => fn())
        this.loading = false
        this.previewInProgress = false
      }
    },

    async cancelPreview() {
      if (!this.previewInProgress) return
      try {
        const invoke = await getInvoke()
        await invoke('cancel_preview')
      } catch (error) {
        console.error('Failed to cancel preview:', error)
      }
    },

    async cancelPreview() {
      if (!this.previewInProgress) return
      try {
        const invoke = await getInvoke()
        await invoke('cancel_preview')
      } catch (error) {
        console.error('Failed to cancel preview:', error)
      }
    },

    async applyExecuteResults(results) {
      const mappings = results
        .filter((r) => r.success)
        .map((r) => ({ original: r.original, new_name: r.new_name }))

      if (mappings.length > 0) {
        this.undoStack.push({
          files: [...this.files],
          mappings,
          basePath: this.basePath,
          timestamp: new Date().toISOString(),
        })
      }

      this.history.unshift({
        timestamp: new Date().toISOString(),
        files: [...this.files],
        script: this.script,
        results,
      })
      await this.persistHistory()

      const successCount = results.filter((r) => r.success).length
      if (successCount > 0) {
        results.forEach((result) => {
          if (result.success) {
            const fileIndex = this.files.findIndex(
              (f) => f.name === result.original || f.path === result.original
            )
            if (fileIndex !== -1) {
              const newName = result.new_name.split(/[/\\]/).pop() || result.new_name
              this.files[fileIndex].name = newName
              if (this.files[fileIndex].path.includes(result.original)) {
                this.files[fileIndex].path = this.files[fileIndex].path.replace(
                  result.original,
                  result.new_name
                )
              }
            }
          }
        })
        if (this.basePath) {
          await this.refreshFolderFileCount()
        }
      }

      const errors = results.filter((r) => !r.success)
      if (errors.length > 0) {
        this.error = `部分文件重命名失败: ${errors.map((e) => e.error).join('; ')}`
      } else {
        this.error = null
      }

      this.invalidatePreview()
      return { confirmed: true, executed: true, results }
    },

    async executeRename(skipConfirm = false) {
      if (!this.hasFiles || !this.scriptValid) {
        this.error = '请先选择文件并编写脚本'
        return { confirmed: false, executed: false }
      }

      if (this.previewStale) {
        await this.previewRename()
      }

      if (!this.hasPreview) {
        await this.previewRename()
        if (!this.hasPreview) {
          return { confirmed: false, executed: false }
        }
      }

      if (!skipConfirm) {
        return { confirmed: false, executed: false, needsConfirm: true }
      }

      this.loading = true
      this.executeInProgress = true
      this.error = null
      this.executeProgress = { completed: 0, total: 0, currentPath: null, phase: '' }

      const { listen } = await import('@tauri-apps/api/event')
      const unlisteners = []

      const onProgress = (event) => {
        const p = event.payload || {}
        this.executeProgress = {
          completed: p.completed ?? 0,
          total: p.total ?? 0,
          currentPath: p.currentPath ?? p.current_path ?? null,
          phase: p.phase ?? '',
        }
      }

      let completeResolver = () => {}
      const completePromise = new Promise((resolve) => {
        completeResolver = resolve
      })

      const onComplete = async (event) => {
        const payload = event.payload || {}
        if (payload.error) {
          this.error = `执行重命名失败: ${payload.error}`
          completeResolver()
          return
        }
        const results = payload.results || []
        await this.applyExecuteResults(results)
        completeResolver()
      }

      try {
        unlisteners.push(await listen('execute-progress', onProgress))
        unlisteners.push(await listen('execute-complete', onComplete))

        const invoke = await getInvoke()
        const fileNames = this.basePath
          ? [this.basePath]
          : this.files.map((f) => f.path)
        const params = this.getCurrentScriptParams()

        let invokeError = null
        const invokePromise = invoke('execute_rename', {
          files: fileNames,
          script: this.script,
          basePath: null,
          params: params && Object.keys(params).length > 0 ? params : null,
          silent: false,
        }).catch((e) => {
          invokeError = e
          completeResolver()
          return null
        })

        await completePromise
        await invokePromise
        if (invokeError) {
          throw invokeError
        }

        return { confirmed: true, executed: true }
      } catch (error) {
        completeResolver()
        this.error = `执行重命名失败: ${error}`
        console.error(error)
        return { confirmed: true, executed: false }
      } finally {
        unlisteners.forEach((fn) => fn())
        this.loading = false
        this.executeInProgress = false
        this.executeProgress = { completed: 0, total: 0, currentPath: null, phase: '' }
      }
    },

    async undo() {
      if (!this.canUndo) {
        this.error = '没有可撤销的操作'
        return
      }

      const lastOperation = this.undoStack.pop()
      if (!lastOperation) return

      this.loading = true
      this.error = null

      try {
        const mappingObj = Object.fromEntries(
          lastOperation.mappings.map((m) => [m.new_name, m.original])
        )
        const filesToRename = lastOperation.mappings.map((m) => m.new_name)
        const script = `function rename() {
  const mapping = ${JSON.stringify(mappingObj)};
  if (mapping[__filePath]) {
    return { [__filePath]: mapping[__filePath] };
  }
  return { [__filePath]: __fileName };
}`

        const invoke = await getInvoke()
        const results = await invoke('execute_rename', {
          files: filesToRename,
          script,
          basePath: lastOperation.basePath || this.basePath,
          silent: true,
        })

        const resultList = results || []
        resultList.forEach((result) => {
          if (result.success) {
            const fileIndex = this.files.findIndex(
              (f) => f.name === result.original.split(/[/\\]/).pop() || f.path === result.original
            )
            if (fileIndex !== -1) {
              const newName = result.new_name.split(/[/\\]/).pop() || result.new_name
              this.files[fileIndex].name = newName
              this.files[fileIndex].path = result.new_name
            }
          }
        })

        this.history.unshift({
          timestamp: new Date().toISOString(),
          files: [...this.files],
          script: '撤销操作',
          results: resultList,
        })
        await this.persistHistory()
        this.invalidatePreview()
      } catch (error) {
        this.error = `撤销失败: ${error}`
        console.error(error)
        this.undoStack.push(lastOperation)
      } finally {
        this.loading = false
      }
    },

    clearError() {
      this.error = null
    },

    setPreviewMd5Mode(mode) {
      if (!['auto', 'always', 'skip'].includes(mode)) return
      this.previewMd5Mode = mode
      try {
        localStorage.setItem(PREVIEW_MD5_MODE_KEY, mode)
      } catch {
        // ignore
      }
      this.invalidatePreview()
    },

    toggleHistoryDrawer() {
      this.showHistoryDrawer = !this.showHistoryDrawer
    },

    async saveScript(name, script, scriptId = null) {
      try {
        const invoke = await getInvoke()
        await invoke('save_script', { name, script, scriptId })
        await this.initSavedScripts()
        return true
      } catch (error) {
        this.error = `保存脚本失败: ${error}`
        console.error(error)
        return false
      }
    },

    loadScript(scriptId) {
      const script = this.savedScripts.find((s) => s.id === scriptId)
      if (script) {
        this.script = script.script
        this.currentScriptId = scriptId
        this.invalidatePreview()

        if (script.parameters && Array.isArray(script.parameters)) {
          if (!this.scriptParams[scriptId]) {
            this.scriptParams[scriptId] = {}
          }
          script.parameters.forEach((param) => {
            if (param.default !== undefined && param.default !== null) {
              this.scriptParams[scriptId][param.name] = param.default
            } else if (
              param.required &&
              this.scriptParams[scriptId][param.name] === undefined
            ) {
              if (param.type === 'number') {
                this.scriptParams[scriptId][param.name] = 0
              } else if (param.type === 'boolean') {
                this.scriptParams[scriptId][param.name] = false
              } else {
                this.scriptParams[scriptId][param.name] = ''
              }
            }
          })
        }
        return true
      }
      return false
    },

    setScriptParam(scriptId, paramName, value) {
      if (!this.scriptParams[scriptId]) {
        this.scriptParams[scriptId] = {}
      }
      this.scriptParams[scriptId][paramName] = value
      this.invalidatePreview()
    },

    getCurrentScriptParams() {
      if (!this.currentScriptId) return null
      return this.scriptParams[this.currentScriptId] || {}
    },

    async deleteScript(scriptId) {
      try {
        const invoke = await getInvoke()
        await invoke('delete_script', { scriptId })

        if (this.currentScriptId === scriptId) {
          this.currentScriptId = null
        }

        const index = this.savedScripts.findIndex((s) => s.id === scriptId)
        if (index !== -1) {
          this.savedScripts.splice(index, 1)
        }
        return true
      } catch (error) {
        this.error = `删除脚本失败: ${error}`
        console.error(error)
        return false
      }
    },

    async renameScript(scriptId, newName) {
      try {
        const invoke = await getInvoke()
        await invoke('rename_script', { scriptId, newName })
        return true
      } catch (error) {
        this.error = `重命名脚本失败: ${error}`
        console.error(error)
        return false
      }
    },

    clearHistory() {
      this.history = []
      this.undoStack = []
      this.persistHistory()
    },
  },
})
