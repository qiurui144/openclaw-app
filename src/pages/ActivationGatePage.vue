<template>
  <WizardLayout :show-footer="false">
    <div class="activation-gate">
      <h2>关注公众号解锁安装</h2>
      <p class="desc">使用微信扫描下方二维码，关注公众号后自动继续安装。</p>

      <div class="qr-container" v-if="qrUrl">
        <img :src="qrUrl" alt="公众号二维码" class="qr-image" />
        <p class="qr-hint">微信扫码 → 关注公众号</p>
        <p class="countdown" v-if="remaining > 0">
          {{ Math.floor(remaining / 60) }}:{{ String(remaining % 60).padStart(2, '0') }} 后过期
        </p>
        <p class="countdown expired" v-else-if="isExpired">
          二维码已过期
          <button class="btn-refresh" @click="fetchQrCode">重新获取</button>
        </p>
      </div>

      <div class="qr-container loading" v-else-if="loading">
        <span>⏳ 正在获取二维码…</span>
      </div>

      <div class="qr-container error" v-else-if="error">
        <p class="err-msg">{{ error }}</p>
        <button class="btn-refresh" @click="fetchQrCode">重试</button>
      </div>

      <div class="status" v-if="verified">
        <span class="ok">✓ 验证通过，正在跳转…</span>
      </div>
    </div>
  </WizardLayout>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import WizardLayout from "@/components/WizardLayout.vue";
import { useWizardNav } from "@/composables/useWizardNav";
import { tauri } from "@/composables/useTauri";

const { next } = useWizardNav();

const qrUrl = ref("");
const ticket = ref("");
const loading = ref(false);
const error = ref<string | null>(null);
const verified = ref(false);
const isExpired = ref(false);
const remaining = ref(0);

let pollTimer: ReturnType<typeof setInterval> | null = null;
let countdownTimer: ReturnType<typeof setInterval> | null = null;
let expiresAt = 0;

onMounted(async () => {
  // 已激活 → 自动跳过
  try {
    const activated = await tauri.checkActivationStatus();
    if (activated) {
      next();
      return;
    }
  } catch {
    // 检查失败时继续显示二维码
  }

  await fetchQrCode();
});

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
  if (countdownTimer) clearInterval(countdownTimer);
});

async function fetchQrCode() {
  loading.value = true;
  error.value = null;
  isExpired.value = false;
  qrUrl.value = "";

  try {
    const result = await tauri.requestActivationQr();
    qrUrl.value = result.qr_url;
    ticket.value = result.ticket;
    expiresAt = Date.now() + result.expires_in * 1000;
    remaining.value = result.expires_in;

    // 启动倒计时
    if (countdownTimer) clearInterval(countdownTimer);
    countdownTimer = setInterval(() => {
      remaining.value = Math.max(0, Math.floor((expiresAt - Date.now()) / 1000));
      if (remaining.value <= 0) {
        isExpired.value = true;
        if (countdownTimer) clearInterval(countdownTimer);
        if (pollTimer) clearInterval(pollTimer);
      }
    }, 1000);

    // 启动轮询（每 2 秒）
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(pollStatus, 2000);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function pollStatus() {
  if (!ticket.value || verified.value || isExpired.value) return;

  try {
    const result = await tauri.pollActivation(ticket.value);
    if (result.verified) {
      verified.value = true;
      if (pollTimer) clearInterval(pollTimer);
      if (countdownTimer) clearInterval(countdownTimer);
      // 短暂延迟后跳转，让用户看到成功提示
      setTimeout(() => next(), 800);
    } else if (result.expired) {
      isExpired.value = true;
      if (pollTimer) clearInterval(pollTimer);
      if (countdownTimer) clearInterval(countdownTimer);
    }
  } catch {
    // 轮询失败静默重试
  }
}
</script>

<style scoped>
.activation-gate { display: flex; flex-direction: column; gap: 16px; align-items: center; text-align: center; }
h2 { font-size: 20px; font-weight: 700; }
.desc { color: var(--color-muted); max-width: 400px; }

.qr-container {
  display: flex; flex-direction: column; align-items: center; gap: 12px;
  background: #f8fafc; border: 1px solid var(--color-border);
  border-radius: var(--radius); padding: 24px 32px;
  min-height: 280px; justify-content: center;
}
.qr-container.loading,
.qr-container.error { color: var(--color-muted); }

.qr-image { width: 200px; height: 200px; border-radius: 8px; }
.qr-hint { color: #475569; font-size: 14px; }
.countdown { color: var(--color-muted); font-size: 12px; font-variant-numeric: tabular-nums; }
.countdown.expired { color: var(--color-error); display: flex; align-items: center; gap: 8px; }

.btn-refresh {
  padding: 4px 12px; border: 1px solid var(--color-border); border-radius: var(--radius);
  background: white; cursor: pointer; font-size: 12px;
}
.btn-refresh:hover { background: #f1f5f9; }

.err-msg { color: var(--color-error); font-size: 14px; }

.status { margin-top: 8px; }
.ok { color: var(--color-success); font-weight: 600; font-size: 16px; }
</style>
