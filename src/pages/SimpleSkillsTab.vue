<template>
  <div class="skills-tab">
    <div class="skills-header">
      <h3>已安装的 Skills</h3>
      <div class="header-actions">
        <button class="btn-secondary" @click="checkUpdates" :disabled="checking">
          {{ checking ? "检查中…" : "🔄 检查更新" }}
        </button>
      </div>
    </div>

    <!-- 网络状态提示 -->
    <div class="network-warn" v-if="networkStatus === 'offline'">
      🌐 网络不可用，使用本地版本。连网后可检查更新。
    </div>

    <!-- 加载中 -->
    <div class="loading" v-if="loading">加载中…</div>

    <!-- Skills 列表 -->
    <div class="skill-list" v-if="!loading">
      <div v-if="skills.length === 0" class="empty">未安装任何 Skills</div>

      <div v-for="skill in skills" :key="skill.name" class="skill-item">
        <div class="skill-info">
          <span class="skill-name">{{ skill.name }}</span>
          <span class="skill-version">v{{ skill.current_version }}</span>
          <span v-if="skill.update_available" class="update-badge">
            可更新至 v{{ skill.latest_version }}
          </span>
        </div>
        <div class="skill-actions">
          <label class="toggle-switch" :title="isEnabled(skill.name) ? '禁用' : '启用'">
            <input
              type="checkbox"
              :checked="isEnabled(skill.name)"
              @change="toggleSkill(skill.name)"
            />
            <span class="toggle-slider"></span>
          </label>
          <button
            v-if="skill.update_available"
            class="btn-sm"
            @click="updateSkill(skill.name)"
            :disabled="updatingSkill === skill.name"
          >
            {{ updatingSkill === skill.name ? "更新中…" : "更新" }}
          </button>
        </div>
      </div>
    </div>

    <!-- 批量更新 -->
    <div class="batch-update" v-if="updatableCount > 0">
      <button class="btn-primary" @click="updateAll" :disabled="updating">
        {{ updating ? "更新中…" : `全部更新（${updatableCount}）` }}
      </button>
    </div>

    <!-- 反馈消息 -->
    <div v-if="msg" class="msg" :class="msg.ok ? 'ok' : 'fail'">{{ msg.text }}</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { tauri, type SkillInfo } from "@/composables/useTauri";

const skills = ref<SkillInfo[]>([]);
const loading = ref(true);
const checking = ref(false);
const updating = ref(false);
const updatingSkill = ref<string | null>(null);
const networkStatus = ref<"online" | "offline" | "unknown">("unknown");
const disabledSkills = ref<Set<string>>(new Set());
const msg = ref<{ ok: boolean; text: string } | null>(null);

const updatableCount = computed(() => skills.value.filter((s) => s.update_available).length);

onMounted(async () => {
  await loadSkills();
  await loadDisabledList();
});

async function loadSkills() {
  loading.value = true;
  try {
    const status = await tauri.getGatewayStatus();
    if (status.install_path) {
      skills.value = await tauri.listSkills(status.install_path);
      networkStatus.value = "online";
    }
  } catch {
    networkStatus.value = "offline";
  }
  loading.value = false;
}

async function loadDisabledList() {
  try {
    const cfg = await tauri.readOpenclawConfig();
    const disabled = (cfg as Record<string, unknown>).disabledSkills;
    if (Array.isArray(disabled)) {
      disabledSkills.value = new Set(disabled as string[]);
    }
  } catch { /* ignore */ }
}

function isEnabled(name: string): boolean {
  return !disabledSkills.value.has(name);
}

async function toggleSkill(name: string) {
  if (disabledSkills.value.has(name)) {
    disabledSkills.value.delete(name);
  } else {
    disabledSkills.value.add(name);
  }
  try {
    await tauri.writeOpenclawConfig({
      disabledSkills: Array.from(disabledSkills.value),
    });
    showMsg(true, `${name} 已${isEnabled(name) ? "启用" : "禁用"}`);
  } catch (e) {
    showMsg(false, `操作失败: ${e}`);
  }
}

async function checkUpdates() {
  checking.value = true;
  try {
    const status = await tauri.getGatewayStatus();
    if (status.install_path) {
      skills.value = await tauri.listSkills(status.install_path);
      networkStatus.value = "online";
      const count = skills.value.filter((s) => s.update_available).length;
      showMsg(true, count > 0 ? `${count} 个 Skills 可更新` : "所有 Skills 已是最新版本");
    }
  } catch {
    networkStatus.value = "offline";
    showMsg(false, "网络不可用，无法检查更新");
  }
  checking.value = false;
}

async function updateSkill(name: string) {
  updatingSkill.value = name;
  try {
    const status = await tauri.getGatewayStatus();
    await tauri.updateSkills(status.install_path, [name]);
    await loadSkills();
    showMsg(true, `${name} 更新成功`);
  } catch (e) {
    showMsg(false, `更新失败: ${e}`);
  }
  updatingSkill.value = null;
}

async function updateAll() {
  updating.value = true;
  try {
    const status = await tauri.getGatewayStatus();
    const names = skills.value.filter((s) => s.update_available).map((s) => s.name);
    await tauri.updateSkills(status.install_path, names);
    await loadSkills();
    showMsg(true, "全部更新完成");
  } catch (e) {
    showMsg(false, `更新失败: ${e}`);
  }
  updating.value = false;
}

function showMsg(ok: boolean, text: string) {
  msg.value = { ok, text };
  setTimeout(() => { msg.value = null; }, 3000);
}
</script>

<style scoped>
.skills-tab { display: flex; flex-direction: column; gap: 14px; }

.skills-header { display: flex; align-items: center; justify-content: space-between; }
.skills-header h3 { font-size: 15px; font-weight: 700; }

.network-warn {
  background: #fffbeb; border: 1px solid #fde68a;
  border-radius: var(--radius); padding: 10px 14px;
  font-size: 12px; color: #92400e;
}

.loading, .empty { text-align: center; padding: 32px; color: var(--color-muted); font-size: 13px; }

.skill-list { display: flex; flex-direction: column; gap: 8px; }

.skill-item {
  display: flex; align-items: center; justify-content: space-between;
  padding: 12px 16px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.skill-info { display: flex; align-items: center; gap: 10px; }
.skill-name { font-size: 13px; font-weight: 600; }
.skill-version { font-size: 12px; color: var(--color-muted); }
.update-badge {
  font-size: 10px; padding: 2px 8px; border-radius: 10px;
  background: #dbeafe; color: #1e40af;
}

.skill-actions { display: flex; align-items: center; gap: 10px; }

.btn-sm {
  padding: 4px 12px; font-size: 11px;
  background: var(--color-primary); color: #fff;
  border: none; border-radius: var(--radius); cursor: pointer;
}
.btn-sm:disabled { opacity: 0.4; }

/* Toggle switch */
.toggle-switch { position: relative; display: inline-block; width: 36px; height: 20px; }
.toggle-switch input { opacity: 0; width: 0; height: 0; }
.toggle-slider {
  position: absolute; cursor: pointer; inset: 0;
  background: #cbd5e1; border-radius: 20px;
  transition: 0.2s;
}
.toggle-slider::before {
  content: ""; position: absolute;
  height: 16px; width: 16px; left: 2px; bottom: 2px;
  background: white; border-radius: 50%;
  transition: 0.2s;
}
.toggle-switch input:checked + .toggle-slider { background: var(--color-primary); }
.toggle-switch input:checked + .toggle-slider::before { transform: translateX(16px); }

.batch-update { display: flex; justify-content: flex-end; }

.msg {
  position: fixed; bottom: 24px; right: 24px;
  padding: 10px 20px; border-radius: var(--radius);
  font-size: 13px; font-weight: 500; box-shadow: var(--shadow);
}
.msg.ok { background: #dcfce7; color: #166534; }
.msg.fail { background: #fef2f2; color: #991b1b; }
</style>
