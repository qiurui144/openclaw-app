<template>
  <Teleport to="body">
    <div class="overlay" @click.self="$emit('close')">
      <div class="modal">
        <h3>扫码前往配置页面</h3>
        <canvas ref="qrCanvas" />
        <p class="url-text">{{ url }}</p>
        <button class="btn-secondary" @click="$emit('close')">关闭</button>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import QRCode from "qrcode";

const props = defineProps<{ url: string }>();
defineEmits<{ close: [] }>();

const qrCanvas = ref<HTMLCanvasElement | null>(null);

onMounted(async () => {
  if (qrCanvas.value) {
    await QRCode.toCanvas(qrCanvas.value, props.url, { width: 200, margin: 2 });
  }
});
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
  padding: 24px;
  display: flex; flex-direction: column; align-items: center; gap: 12px;
  box-shadow: 0 20px 60px rgba(0,0,0,.2);
}
.url-text { font-size: 11px; color: var(--color-muted); word-break: break-all; max-width: 220px; text-align: center; }
</style>
