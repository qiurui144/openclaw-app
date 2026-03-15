<template>
  <div class="skill-card" :class="{ paid: skill.is_paid }">
    <div class="skill-header">
      <div class="skill-icon">{{ skill.icon || (skill.is_paid ? '💎' : '🧩') }}</div>
      <div class="skill-info">
        <div class="skill-name">{{ skill.name }}</div>
        <div class="skill-author">{{ skill.author }}</div>
      </div>
      <div class="skill-badge" v-if="skill.is_paid">
        <span class="price-tag">{{ skill.price_label || `¥${skill.price}` }}</span>
      </div>
      <div class="skill-badge free-badge" v-else>
        <span>免费</span>
      </div>
    </div>

    <p class="skill-desc">{{ skill.description }}</p>

    <div class="skill-footer">
      <span class="skill-version">v{{ skill.version }}</span>
      <span class="skill-category">{{ skill.category }}</span>

      <!-- 免费 skill：已内置 -->
      <span v-if="!skill.is_paid" class="status-label installed">已内置</span>

      <!-- 付费 skill：根据状态显示不同按钮 -->
      <template v-else>
        <!-- 已安装 -->
        <div v-if="isInstalled" class="action-group">
          <span class="status-label installed">已安装</span>
          <button class="btn-sm btn-danger" @click="$emit('uninstall', skill.slug)" :disabled="operating">
            卸载
          </button>
        </div>
        <!-- 有权但未安装 -->
        <button
          v-else-if="hasAccess"
          class="btn-sm btn-primary"
          @click="$emit('install', skill.slug)"
          :disabled="operating"
        >
          {{ operating ? '安装中…' : '安装' }}
        </button>
        <!-- 无权限 -->
        <button
          v-else
          class="btn-sm btn-upgrade"
          @click="$emit('purchase', skill)"
        >
          购买
        </button>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SkillEntry } from "@/stores/license";

defineProps<{
  skill: SkillEntry;
  isInstalled: boolean;
  hasAccess: boolean;
  operating: boolean;
}>();

defineEmits<{
  install: [slug: string];
  uninstall: [slug: string];
  purchase: [skill: SkillEntry];
}>();
</script>

<style scoped>
.skill-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 14px 16px;
  display: flex; flex-direction: column; gap: 8px;
  transition: border-color .15s;
}
.skill-card:hover { border-color: var(--color-primary); }
.skill-card.paid { border-left: 3px solid #f59e0b; }
.skill-header { display: flex; align-items: center; gap: 10px; }
.skill-icon { font-size: 24px; flex-shrink: 0; }
.skill-info { flex: 1; min-width: 0; }
.skill-name { font-size: 14px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.skill-author { font-size: 11px; color: var(--color-muted); }
.skill-badge { font-size: 11px; padding: 2px 8px; border-radius: 10px; flex-shrink: 0; }
.skill-badge.free-badge { background: #f0fdf4; color: #166534; }
.skill-card.paid .skill-badge { background: #fffbeb; color: #92400e; }
.price-tag { font-weight: 600; }
.skill-desc { font-size: 12px; color: #475569; line-height: 1.5; margin: 0; }
.skill-footer {
  display: flex; align-items: center; gap: 8px;
  font-size: 11px; color: var(--color-muted);
  padding-top: 6px; border-top: 1px solid var(--color-border);
}
.skill-version { font-family: monospace; }
.skill-category {
  background: var(--color-bg); padding: 1px 6px;
  border-radius: 4px;
}
.status-label { margin-left: auto; }
.status-label.installed { color: var(--color-success); font-weight: 500; }
.action-group { margin-left: auto; display: flex; align-items: center; gap: 6px; }
.btn-sm {
  margin-left: auto;
  padding: 3px 10px; font-size: 11px;
  border: none; border-radius: 4px; cursor: pointer;
}
.btn-sm.btn-primary { background: var(--color-primary); color: #fff; }
.btn-sm.btn-primary:hover:not(:disabled) { opacity: .9; }
.btn-sm.btn-upgrade { background: #f59e0b; color: #fff; }
.btn-sm.btn-upgrade:hover { opacity: .9; }
.btn-sm.btn-danger { background: transparent; color: var(--color-error); border: 1px solid var(--color-error); }
.btn-sm.btn-danger:hover:not(:disabled) { background: #fef2f2; }
.btn-sm:disabled { opacity: .45; cursor: not-allowed; }
</style>
