import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ScriptEditorWindow from './components/ScriptEditorWindow.vue'
import './style.css'

const app = createApp(ScriptEditorWindow)
const pinia = createPinia()

app.use(pinia)
app.mount('#app')
