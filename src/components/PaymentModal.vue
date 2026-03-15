<template>
  <Teleport to="body">
    <div class="overlay" @click.self="$emit('close')">
      <div class="modal">
        <h3>{{ title }}</h3>
        <p class="amount">¥{{ order?.amount ?? '—' }}</p>

        <div v-if="loading" class="loading">创建订单中…</div>

        <div v-else-if="order" class="qr-section">
          <canvas ref="qrCanvas" />
          <p class="hint">请使用微信或支付宝扫码支付</p>
          <div class="status-row" :class="payStatus">
            <span v-if="payStatus === 'pending'">等待支付…</span>
            <span v-else-if="payStatus === 'paid'">支付成功！</span>
            <span v-else-if="payStatus === 'failed'">支付失败，请重试</span>
          </div>
        </div>

        <p v-if="error" class="error-msg">{{ error }}</p>

        <div class="btn-row">
          <button class="btn-secondary" @click="$emit('close')">
            {{ payStatus === 'paid' ? '完成' : '取消' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import QRCode from "qrcode";
import { useLicenseStore, type PaymentOrder } from "@/stores/license";

const props = defineProps<{
  title: string;
  plan: string;
  skillSlug?: string;
}>();
const emit = defineEmits<{ close: []; success: [] }>();

const license = useLicenseStore();
const qrCanvas = ref<HTMLCanvasElement | null>(null);
const order = ref<PaymentOrder | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const payStatus = ref<"pending" | "paid" | "failed">("pending");

let pollTimer: ReturnType<typeof setInterval> | null = null;

onMounted(async () => {
  try {
    order.value = await license.createPayment(props.plan, props.skillSlug);
    loading.value = false;

    // 渲染二维码
    await renderQr();

    // 轮询支付状态（每 3 秒）
    pollTimer = setInterval(async () => {
      if (!order.value) return;
      try {
        const status = await license.checkPayment(order.value.order_id);
        if (status === "paid") {
          payStatus.value = "paid";
          if (pollTimer) clearInterval(pollTimer);
          // 刷新许可证
          await license.refresh();
          emit("success");
        } else if (status === "failed" || status === "expired") {
          payStatus.value = "failed";
          if (pollTimer) clearInterval(pollTimer);
        }
      } catch { /* 静默 */ }
    }, 3000);
  } catch (e) {
    loading.value = false;
    error.value = e instanceof Error ? e.message : String(e);
  }
});

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
});

watch(qrCanvas, () => renderQr());

async function renderQr() {
  if (qrCanvas.value && order.value?.qr_url) {
    await QRCode.toCanvas(qrCanvas.value, order.value.qr_url, { width: 200, margin: 2 });
  }
}
</script>

<style scoped>
.overlay {
  position: fixed; inset: 0;
  background: rgba(0,0,0,.5);
  display: flex; align-items: center; justify-content: center;
  z-index: 999;
}
.modal {
  background: var(--color-surface);
  border-radius: var(--radius);
  padding: 28px 32px;
  width: 340px;
  display: flex; flex-direction: column; align-items: center; gap: 14px;
  box-shadow: 0 20px 60px rgba(0,0,0,.2);
}
h3 { font-size: 18px; font-weight: 700; margin: 0; }
.amount { font-size: 32px; font-weight: 700; color: var(--color-primary); margin: 0; }
.loading { font-size: 13px; color: var(--color-muted); }
.qr-section { display: flex; flex-direction: column; align-items: center; gap: 8px; }
.hint { font-size: 12px; color: var(--color-muted); margin: 0; }
.status-row { font-size: 13px; font-weight: 500; }
.status-row.pending { color: var(--color-warning); }
.status-row.paid { color: var(--color-success); }
.status-row.failed { color: var(--color-error); }
.error-msg { color: var(--color-error); font-size: 12px; margin: 0; }
.btn-row { width: 100%; display: flex; justify-content: center; }
</style>
