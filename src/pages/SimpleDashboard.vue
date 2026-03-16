<template>
  <div class="simple-dashboard">
    <header class="sd-header">
      <span class="sd-brand">🦾 OpenClaw 配置</span>
      <div class="sd-tabs">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="sd-tab"
          :class="{ active: activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ tab.icon }} {{ tab.label }}
        </button>
      </div>
    </header>
    <main class="sd-body">
      <KeepAlive>
        <component :is="tabComponents[activeTab]" />
      </KeepAlive>
    </main>
  </div>
</template>

<script setup lang="ts">
import { ref, type Component } from "vue";
import SimpleStatusTab from "./SimpleStatusTab.vue";
import SimpleConfigTab from "./SimpleConfigTab.vue";
import SimpleSkillsTab from "./SimpleSkillsTab.vue";
import SimpleChatTab from "./SimpleChatTab.vue";

type TabKey = "status" | "config" | "skills" | "chat";

const activeTab = ref<TabKey>("status");

const tabComponents: Record<TabKey, Component> = {
  status: SimpleStatusTab,
  config: SimpleConfigTab,
  skills: SimpleSkillsTab,
  chat: SimpleChatTab,
};

const tabs: { key: TabKey; label: string; icon: string }[] = [
  { key: "status", label: "状态", icon: "📊" },
  { key: "config", label: "配置", icon: "⚙" },
  { key: "skills", label: "Skills", icon: "🧩" },
  { key: "chat", label: "聊天", icon: "💬" },
];
</script>

<style scoped>
.simple-dashboard {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--color-bg);
}

.sd-header {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 12px 24px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
}

.sd-brand {
  font-weight: 700;
  font-size: 15px;
  white-space: nowrap;
}

.sd-tabs {
  display: flex;
  gap: 0;
}

.sd-tab {
  padding: 8px 18px;
  font-size: 13px;
  font-weight: 500;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  cursor: pointer;
  color: var(--color-muted);
  transition: color 0.15s, border-color 0.15s;
}

.sd-tab:hover {
  color: var(--color-text);
}

.sd-tab.active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}

.sd-body {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}
</style>
