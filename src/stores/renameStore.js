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
  // 示例：返回原文件名（不进行修改）
  return __fileName;
}`,
    previewResults: [], // 预览结果 [{ original, new_name, error }]
    history: [], // 历史记录 [{ timestamp, files, script, results }]
    undoStack: [], // 撤销栈 [{ files, mappings }]
    basePath: null, // 基础路径（如果选择的是文件夹）
    loading: false,
    error: null,
    savedScripts: [], // 保存的脚本模板 [{ id, name, script, created_at, updated_at }]
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
        const invoke = await getInvoke()
        const selected = await open({
          multiple: false,
          directory: true,
        })

        if (selected) {
          this.basePath = selected
          const files = await invoke('get_folder_files', {
            folderPath: selected,
          })

          this.files = files.map((name) => ({
            name,
            path: `${selected}/${name}`,
            size: 0,
            modified: new Date().toISOString(),
          }))
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
    },

    // 清空文件列表
    clearFiles() {
      this.files = []
      this.previewResults = []
      this.basePath = null
    },

    // 更新脚本
    updateScript(script) {
      this.script = script
      // 脚本变化时清空预览
      this.previewResults = []
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
        // 如果 basePath 存在，传递文件名；否则传递完整路径
        const fileNames = this.basePath 
          ? this.files.map((f) => f.name)
          : this.files.map((f) => f.path)
        const result = await invoke('preview_rename', {
          files: fileNames,
          script: this.script,
        })

        this.previewResults = result.mappings.map(([original, new_name]) => ({
          original,
          new_name,
          error: null,
        }))

        if (result.errors && result.errors.length > 0) {
          this.error = result.errors.join('; ')
        }
      } catch (error) {
        this.error = `预览失败: ${error}`
        console.error(error)
        this.previewResults = []
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
        // 如果 basePath 存在，传递文件名；否则传递完整路径
        const fileNames = this.basePath 
          ? this.files.map((f) => f.name)
          : this.files.map((f) => f.path)
        const results = await invoke('execute_rename', {
          files: fileNames,
          script: this.script,
          basePath: this.basePath,
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

    // 保存脚本模板
    async saveScript(name, script, scriptId = null) {
      try {
        const invoke = await getInvoke()
        const saved = await invoke('save_script', {
          name,
          script,
          scriptId,
        })
        
        // 更新本地状态
        const index = this.savedScripts.findIndex((s) => s.id === saved.id)
        if (index !== -1) {
          this.savedScripts[index] = saved
        } else {
          this.savedScripts.push(saved)
        }
        
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
        this.previewResults = [] // 清空预览
        return true
      }
      return false
    },

    // 删除脚本模板
    async deleteScript(scriptId) {
      try {
        const invoke = await getInvoke()
        await invoke('delete_script', { scriptId })
        
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
        
        // 更新本地状态
        const script = this.savedScripts.find((s) => s.id === scriptId)
        if (script) {
          script.name = newName.trim()
          script.updated_at = new Date().toISOString()
        }
        
        return true
      } catch (error) {
        this.error = `重命名脚本失败: ${error}`
        console.error(error)
        return false
      }
    },
  },
})
