<template>
  <div class="chat-tab">
    <!-- 连接状态 -->
    <div class="chat-status" :class="connected ? 'online' : 'offline'">
      <span class="dot"></span>
      {{ connected ? "已连接" : "未连接" }}
      <button v-if="!connected" class="btn-sm-link" @click="connect">重新连接</button>
    </div>

    <!-- 消息列表 -->
    <div class="messages" ref="messagesRef">
      <div v-if="messages.length === 0" class="empty-chat">
        发送消息开始对话
      </div>
      <div
        v-for="(m, i) in messages"
        :key="i"
        class="message"
        :class="m.role"
      >
        <div class="msg-role">{{ m.role === "user" ? "我" : "AI" }}</div>
        <div class="msg-content">{{ m.content }}</div>
      </div>
      <div v-if="receiving" class="message assistant">
        <div class="msg-role">AI</div>
        <div class="msg-content typing">{{ streamContent || "思考中…" }}</div>
      </div>
    </div>

    <!-- 输入区 -->
    <div class="input-bar">
      <textarea
        v-model="input"
        placeholder="输入消息，Enter 发送，Shift+Enter 换行"
        @keydown.enter.exact.prevent="send"
        :disabled="!connected || receiving"
        rows="2"
      />
      <button class="btn-primary send-btn" @click="send" :disabled="!connected || !input.trim() || receiving">
        发送
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, onActivated, onDeactivated, nextTick, watch } from "vue";
import { tauri } from "@/composables/useTauri";

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

const messages = ref<ChatMessage[]>([]);
const input = ref("");
const connected = ref(false);
const receiving = ref(false);
const streamContent = ref("");
const messagesRef = ref<HTMLElement | null>(null);

let ws: WebSocket | null = null;
let port = 18789;
let adminPassword = "";

onMounted(async () => {
  try {
    const status = await tauri.getGatewayStatus();
    port = status.port;
    // 从配置读取密码（用于 WebSocket 认证）
    const cfg = await tauri.readOpenclawConfig();
    const gw = cfg.gateway as Record<string, unknown> | undefined;
    if (gw?.auth && typeof gw.auth === "object") {
      adminPassword = (gw.auth as Record<string, string>).password || "";
    }
    if (status.running) {
      connect();
    }
  } catch { /* ignore */ }
});

onUnmounted(() => { closeWs(); });
onDeactivated(() => { closeWs(); });
onActivated(() => {
  if (!ws && port) { connect(); }
});

function closeWs() {
  if (ws) { ws.close(); ws = null; }
  connected.value = false;
}

function connect() {
  if (ws) {
    ws.close();
  }

  const url = `ws://127.0.0.1:${port}/`;
  ws = new WebSocket(url);

  ws.onopen = () => {
    connected.value = true;
    // 发送认证消息
    if (adminPassword) {
      ws?.send(JSON.stringify({ type: "auth", password: adminPassword }));
    }
  };

  ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      handleWsMessage(data);
    } catch {
      // 非 JSON 消息，可能是纯文本流
      if (receiving.value) {
        streamContent.value += event.data;
      }
    }
  };

  ws.onclose = () => {
    connected.value = false;
  };

  ws.onerror = () => {
    connected.value = false;
  };
}

function handleWsMessage(data: Record<string, unknown>) {
  const type = data.type as string;

  if (type === "chat:stream" || type === "stream") {
    receiving.value = true;
    const content = (data.content || data.text || data.delta || "") as string;
    streamContent.value += content;
  } else if (type === "chat:end" || type === "end") {
    if (streamContent.value) {
      messages.value.push({ role: "assistant", content: streamContent.value });
    }
    streamContent.value = "";
    receiving.value = false;
  } else if (type === "chat:reply" || type === "reply") {
    const content = (data.content || data.text || "") as string;
    messages.value.push({ role: "assistant", content });
    receiving.value = false;
  } else if (type === "error") {
    const errMsg = (data.message || data.error || "未知错误") as string;
    messages.value.push({ role: "assistant", content: `[错误] ${errMsg}` });
    receiving.value = false;
  }
}

function send() {
  const text = input.value.trim();
  if (!text || !ws || !connected.value || receiving.value) return;

  messages.value.push({ role: "user", content: text });
  streamContent.value = "";

  ws.send(JSON.stringify({
    type: "chat",
    content: text,
  }));

  input.value = "";
  receiving.value = true;
}

// 自动滚动到底部
watch(
  () => [messages.value.length, streamContent.value],
  () => {
    nextTick(() => {
      if (messagesRef.value) {
        messagesRef.value.scrollTop = messagesRef.value.scrollHeight;
      }
    });
  },
);
</script>

<style scoped>
.chat-tab {
  display: flex; flex-direction: column;
  height: calc(100vh - 120px);
  gap: 12px;
}

.chat-status {
  display: flex; align-items: center; gap: 8px;
  font-size: 12px; padding: 6px 12px;
  border-radius: var(--radius);
  border: 1px solid var(--color-border);
}
.chat-status .dot { width: 8px; height: 8px; border-radius: 50%; }
.chat-status.online .dot { background: var(--color-success); }
.chat-status.offline .dot { background: var(--color-error); }
.btn-sm-link {
  background: none; border: none; color: var(--color-primary);
  font-size: 12px; cursor: pointer; margin-left: auto;
}

.messages {
  flex: 1; overflow-y: auto;
  display: flex; flex-direction: column; gap: 12px;
  padding: 12px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
}

.empty-chat { text-align: center; color: var(--color-muted); padding: 40px 0; font-size: 13px; }

.message { display: flex; flex-direction: column; gap: 4px; max-width: 80%; }
.message.user { align-self: flex-end; }
.message.assistant { align-self: flex-start; }

.msg-role { font-size: 10px; color: var(--color-muted); }
.message.user .msg-role { text-align: right; }

.msg-content {
  padding: 10px 14px; border-radius: 12px;
  font-size: 13px; line-height: 1.6;
  white-space: pre-wrap; word-break: break-word;
}
.message.user .msg-content {
  background: var(--color-primary); color: #fff;
  border-bottom-right-radius: 4px;
}
.message.assistant .msg-content {
  background: var(--color-bg); border: 1px solid var(--color-border);
  border-bottom-left-radius: 4px;
}
.typing { color: var(--color-muted); }

.input-bar {
  display: flex; gap: 10px;
}
.input-bar textarea {
  flex: 1; padding: 10px 14px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  font-size: 13px; resize: none;
  font-family: inherit;
}
.input-bar textarea:focus { outline: none; border-color: var(--color-primary); }
.send-btn { align-self: flex-end; }
</style>
