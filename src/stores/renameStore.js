import { defineStore } from 'pinia'

// 动态导入 Tauri API 的辅助函数
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

// 从配置文件加载保存的脚本模板
async function loadSavedScripts() {
  try {
    const invoke = await getInvoke()
    return await invoke('load_saved_scripts')
  } catch (error) {
    console.error('Failed to load saved scripts:', error)
    return []
  }
}

export const useRenameStore = defineStore('rename', {
  state: () => ({
    files: [], // 文件列表 [{ name, path, size, modified }]
    script: `function rename() {
  // 返回对象格式：{原始路径: 新路径}
  // 使用 console.log() 来输出日志，日志会在预览面板中显示
  // console.log('处理文件:', __fileName);
  
  // 获取脚本参数（如果定义了参数）
  // var prefix = __params && __params.prefix ? __params.prefix : '';
  // var suffix = __params && __params.suffix ? __params.suffix : '';
  // var maxLength = __params && __params.maxLength ? __params.maxLength : 255;
  
  // 单个文件示例：
  return {[__filePath]: __fileName};
  
  // 使用参数的示例：
  // var newName = prefix + __fileName + suffix;
  // if (newName.length > maxLength) {
  //   newName = newName.substring(0, maxLength);
  // }
  // return {[__filePath]: newName};
  
  // 批量重命名示例（文件夹场景）：
  // var result = {};
  // var files = __utils.fs.readDirFilesRecursive(__filePath);
  // for (var i = 0; i < files.length; i++) {
  //   console.log('处理文件:', files[i]);
  //   result[files[i]] = 'new_name_' + i;
  // }
  // return result;
}`,
    previewResults: [], // 预览结果 [{ original, new_name, error }]
    hasPreviewed: false, // 是否已执行过预览（即使结果为空）
    history: [], // 历史记录 [{ timestamp, files, script, results }]
    undoStack: [], // 撤销栈 [{ files, mappings }]
    basePath: null, // 基础路径（如果选择的是文件夹）
    loading: false,
    error: null,
    savedScripts: [], // 保存的脚本模板 [{ id, name, script, created_at, updated_at, display_name, description, parameters }]
    currentScriptId: null, // 当前使用的脚本 ID
    scriptParams: {}, // 脚本参数值 { scriptId: { paramName: value } }
  }),

  getters: {
    hasFiles: (state) => state.files.length > 0,
    hasPreview: (state) => state.previewResults.length > 0,
    canUndo: (state) => state.undoStack.length > 0,
    scriptValid: (state) => state.script.trim().length > 0,
  },

  actions: {
    // 初始化：加载保存的脚本
    async initSavedScripts() {
      this.savedScripts = await loadSavedScripts()
    },
    // 选择文件
    async selectFiles() {
      try {
        const open = await getOpen()
        const selected = await open({
          multiple: true,
          directory: false,
        })

        if (selected) {
          const files = Array.isArray(selected) ? selected : [selected]
          const fileList = files.map((path) => {
            const name = path.split(/[/\\]/).pop()
            return {
              name,
              path,
              size: 0, // 可以通过 fs API 获取
              modified: new Date().toISOString(),
            }
          })
          this.files = [...this.files, ...fileList]
          this.basePath = null
        }
      } catch (error) {
        this.error = `选择文件失败: ${error}`
        console.error(error)
      }
    },

    // 选择文件夹
    async selectFolder() {
      try {
        const open = await getOpen()
        const selected = await open({
          multiple: false,
          directory: true,
        })

        if (selected) {
          this.basePath = selected
          // 只显示选中的文件夹本身
          // 注意：当选择文件夹时，files 中只存储文件夹名，不存储完整路径
          // 这样在 previewRename 时，会传递 name，然后 Rust 端会用 basePath 拼接
          const folderName = selected.split(/[/\\]/).pop() || selected
          this.files = [{
            name: folderName,
            path: folderName, // 只存储文件夹名，不存储完整路径
            size: 0,
            modified: new Date().toISOString(),
          }]
          console.log('[DEBUG] selectFolder: basePath=', this.basePath, 'folderName=', folderName, 'files=', this.files)
        }
      } catch (error) {
        this.error = `选择文件夹失败: ${error}`
        console.error(error)
      }
    },

    // 移除文件
    removeFile(index) {
      this.files.splice(index, 1)
      this.previewResults = []
      this.hasPreviewed = false
    },

    // 清空文件列表
    clearFiles() {
      this.files = []
      this.previewResults = []
      this.hasPreviewed = false
      this.basePath = null
    },

    // 更新脚本
    updateScript(script) {
      this.script = script
      // 脚本变化时清空预览和当前脚本标识
      this.previewResults = []
      this.hasPreviewed = false
      this.scriptLogs = []
      this.currentScriptId = null // 手动编辑脚本时，清除当前脚本标识
    },

    // 预览重命名
    async previewRename() {
      if (!this.hasFiles || !this.scriptValid) {
        this.error = '请先选择文件并编写脚本'
        return
      }

      this.loading = true
      this.error = null

      try {
        const invoke = await getInvoke()
        // 如果 basePath 存在，直接传递 basePath（文件夹路径）；否则传递文件的完整路径
        const fileNames = this.basePath 
          ? [this.basePath] // 选择文件夹时，直接传递 basePath，不需要拼接
          : this.files.map((f) => f.path)
        console.log('[DEBUG] previewRename: basePath=', this.basePath, 'fileNames=', fileNames)
        // 获取当前脚本的参数值
        const params = this.getCurrentScriptParams()
        
        const result = await invoke('preview_rename', {
          files: fileNames,
          script: this.script,
          basePath: null, // 选择文件夹时，不传递 basePath，因为 fileNames 已经是完整路径
          params: params && Object.keys(params).length > 0 ? params : null,
        })

        this.previewResults = result.mappings.map(([original, new_name]) => ({
          original,
          new_name,
          error: null,
        }))

        // 保存日志
        this.scriptLogs = result.logs || []
        console.log('[DEBUG] Frontend: Received logs:', this.scriptLogs.length, this.scriptLogs)

        // 标记已执行过预览
        this.hasPreviewed = true

        // 处理错误信息
        if (result.errors && result.errors.length > 0) {
          this.error = result.errors.join('\n')
        } else {
          this.error = null
        }
      } catch (error) {
        this.error = `预览失败: ${error}`
        console.error(error)
        this.previewResults = []
        this.hasPreviewed = false
      } finally {
        this.loading = false
      }
    },

    // 执行重命名
    async executeRename() {
      if (!this.hasFiles || !this.scriptValid) {
        this.error = '请先选择文件并编写脚本'
        return
      }

      // 如果没有预览，先执行预览
      if (!this.hasPreview) {
        await this.previewRename()
        if (!this.hasPreview) {
          return
        }
      }

      this.loading = true
      this.error = null

      try {
        const invoke = await getInvoke()
        // 如果 basePath 存在，传递 basePath 本身（因为选择文件夹时，basePath 就是文件夹路径）
        // 否则传递文件的完整路径
        const fileNames = this.basePath 
          ? [this.basePath] // 选择文件夹时，直接传递 basePath，不需要拼接
          : this.files.map((f) => f.path)
        
        // 获取当前脚本的参数值
        const params = this.getCurrentScriptParams()
        const results = await invoke('execute_rename', {
          files: fileNames,
          script: this.script,
          basePath: null, // 选择文件夹时，不传递 basePath，因为 fileNames 已经是完整路径
          params: params && Object.keys(params).length > 0 ? params : null,
        })

        // 保存到撤销栈
        const mappings = results
          .filter((r) => r.success)
          .map((r) => ({ original: r.original, new_name: r.new_name }))
        if (mappings.length > 0) {
          this.undoStack.push({
            files: [...this.files],
            mappings,
            timestamp: new Date().toISOString(),
          })
        }

        // 保存到历史记录
        this.history.unshift({
          timestamp: new Date().toISOString(),
          files: [...this.files],
          script: this.script,
          results,
        })

        // 更新文件列表
        const successCount = results.filter((r) => r.success).length
        if (successCount > 0) {
          // 更新成功重命名的文件
          results.forEach((result) => {
            if (result.success) {
              const fileIndex = this.files.findIndex(
                (f) => f.name === result.original
              )
              if (fileIndex !== -1) {
                this.files[fileIndex].name = result.new_name
                this.files[fileIndex].path = this.files[fileIndex].path.replace(
                  result.original,
                  result.new_name
                )
              }
            }
          })
        }

        // 显示错误信息
        const errors = results.filter((r) => !r.success)
        if (errors.length > 0) {
          this.error = `部分文件重命名失败: ${errors.map((e) => e.error).join('; ')}`
        } else {
          this.error = null
        }

        // 清空预览
        this.previewResults = []
      } catch (error) {
        this.error = `执行重命名失败: ${error}`
        console.error(error)
      } finally {
        this.loading = false
      }
    },

    // 撤销操作
    async undo() {
      if (!this.canUndo) {
        this.error = '没有可撤销的操作'
        return
      }

      const lastOperation = this.undoStack.pop()
      if (!lastOperation) {
        return
      }

      this.loading = true
      this.error = null

      try {
        // 执行反向重命名 - 需要调用 Rust 命令
        const filesToRename = lastOperation.mappings.map((m) => m.new_name)
        const mappingObj = Object.fromEntries(
          lastOperation.mappings.map((m) => [m.new_name, m.original])
        )
        const script = `function rename(filename) {
          const mapping = ${JSON.stringify(mappingObj)};
          return mapping[filename] || filename;
        }`

        const invoke = await getInvoke()
        const results = await invoke('execute_rename', {
          files: filesToRename,
          script,
          basePath: this.basePath,
        })

        // 更新文件列表
        results.forEach((result) => {
          if (result.success) {
            const fileIndex = this.files.findIndex(
              (f) => f.name === result.original
            )
            if (fileIndex !== -1) {
              this.files[fileIndex].name = result.new_name
              this.files[fileIndex].path = this.files[fileIndex].path.replace(
                result.original,
                result.new_name
              )
            }
          }
        })

        // 添加到历史记录
        this.history.unshift({
          timestamp: new Date().toISOString(),
          files: [...this.files],
          script: '撤销操作',
          results,
        })
      } catch (error) {
        this.error = `撤销失败: ${error}`
        console.error(error)
        // 恢复撤销栈
        this.undoStack.push(lastOperation)
      } finally {
        this.loading = false
      }
    },

    // 清空错误
    clearError() {
      this.error = null
    },

    // 保存脚本模板（保存为文件）
    async saveScript(name, script, scriptId = null) {
      try {
        const invoke = await getInvoke()
        const saved = await invoke('save_script', {
          name,
          script,
          scriptId,
        })
        
        // 刷新脚本列表
        await this.initSavedScripts()
        
        return true
      } catch (error) {
        this.error = `保存脚本失败: ${error}`
        console.error(error)
        return false
      }
    },

    // 加载脚本模板
    loadScript(scriptId) {
      const script = this.savedScripts.find((s) => s.id === scriptId)
      if (script) {
        this.script = script.script
        this.currentScriptId = scriptId // 记录当前使用的脚本 ID
        this.previewResults = [] // 清空预览
        this.hasPreviewed = false // 重置预览状态
        
        // 初始化参数默认值
        if (script.parameters && Array.isArray(script.parameters)) {
          if (!this.scriptParams[scriptId]) {
            this.scriptParams[scriptId] = {}
          }
          script.parameters.forEach((param) => {
            if (param.default !== undefined && param.default !== null) {
              this.scriptParams[scriptId][param.name] = param.default
            } else if (param.required && this.scriptParams[scriptId][param.name] === undefined) {
              // 必填参数但没有默认值，初始化为空字符串或0或false
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

    // 设置脚本参数值
    setScriptParam(scriptId, paramName, value) {
      if (!this.scriptParams[scriptId]) {
        this.scriptParams[scriptId] = {}
      }
      this.scriptParams[scriptId][paramName] = value
    },

    // 获取当前脚本的参数值
    getCurrentScriptParams() {
      if (!this.currentScriptId) {
        return null
      }
      return this.scriptParams[this.currentScriptId] || {}
    },

    // 删除脚本模板
    async deleteScript(scriptId) {
      try {
        const invoke = await getInvoke()
        await invoke('delete_script', { scriptId })
        
        // 如果删除的是当前使用的脚本，清除当前脚本标识
        if (this.currentScriptId === scriptId) {
          this.currentScriptId = null
        }
        
        // 更新本地状态
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

    // 重命名脚本模板
    async renameScript(scriptId, newName) {
      try {
        const invoke = await getInvoke()
        await invoke('rename_script', { scriptId, newName })
        
        // 重命名后需要重新加载脚本列表，因为脚本 ID 会改变（基于文件名）
        // 不在这里更新本地状态，而是让调用者重新加载列表
        
        return true
      } catch (error) {
        this.error = `重命名脚本失败: ${error}`
        console.error(error)
        return false
      }
    },
  },
})
