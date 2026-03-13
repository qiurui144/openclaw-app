// Plan B 将实现完整前端
import { createApp } from 'vue'
import { createPinia } from 'pinia'

const app = createApp({ template: '<div>OpenClaw Wizard - Loading...</div>' })
app.use(createPinia())
app.mount('#app')
