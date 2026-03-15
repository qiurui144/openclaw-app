<template>
  <Teleport to="body">
    <div class="overlay" @click.self="$emit('close')">
      <div class="modal">
        <h3>登录 OpenClaw</h3>

        <!-- 模式切换 -->
        <div class="mode-tabs">
          <button :class="{ active: mode === 'phone' }" @click="mode = 'phone'">手机号登录</button>
          <button :class="{ active: mode === 'code' }" @click="mode = 'code'">授权码激活</button>
        </div>

        <!-- 手机号登录 -->
        <template v-if="mode === 'phone'">
          <p class="desc">扫码支付用户，登录后绑定当前设备</p>

          <div class="form-group">
            <label>手机号</label>
            <input
              v-model="phone"
              type="tel"
              placeholder="请输入手机号"
              maxlength="11"
              :disabled="step === 'sms'"
            />
          </div>

          <div v-if="step === 'sms'" class="form-group">
            <label>验证码</label>
            <div class="code-row">
              <input
                v-model="smsCode"
                type="text"
                placeholder="6 位验证码"
                maxlength="6"
                @keyup.enter="doPhoneLogin"
              />
              <button class="btn-secondary resend-btn" :disabled="countdown > 0" @click="sendSms">
                {{ countdown > 0 ? `${countdown}s` : "重发" }}
              </button>
            </div>
          </div>

          <div class="btn-row">
            <button class="btn-secondary" @click="$emit('close')">取消</button>
            <button
              v-if="step === 'phone'"
              class="btn-primary"
              :disabled="!isPhoneValid || sending"
              @click="sendSms"
            >
              {{ sending ? "发送中…" : "获取验证码" }}
            </button>
            <button
              v-else
              class="btn-primary"
              :disabled="smsCode.length < 6 || logging"
              @click="doPhoneLogin"
            >
              {{ logging ? "登录中…" : "登录" }}
            </button>
          </div>
        </template>

        <!-- 授权码激活 -->
        <template v-if="mode === 'code'">
          <p class="desc">输入授权码激活付费 Skills，不限设备</p>

          <div class="form-group">
            <label>授权码</label>
            <input
              v-model="activationCode"
              type="text"
              placeholder="OC-XXXX-XXXX-XXXX"
              maxlength="16"
              class="code-input"
              @keyup.enter="doRedeem"
            />
          </div>

          <div class="btn-row">
            <button class="btn-secondary" @click="$emit('close')">取消</button>
            <button
              class="btn-primary"
              :disabled="!isCodeValid || logging"
              @click="doRedeem"
            >
              {{ logging ? "激活中…" : "激活" }}
            </button>
          </div>
        </template>

        <p v-if="error" class="error-msg">{{ error }}</p>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { useLicenseStore } from "@/stores/license";

const emit = defineEmits<{ close: []; success: [] }>();
const license = useLicenseStore();

const mode = ref<"phone" | "code">("phone");
const step = ref<"phone" | "sms">("phone");
const phone = ref("");
const smsCode = ref("");
const activationCode = ref("");
const sending = ref(false);
const logging = ref(false);
const countdown = ref(0);
const error = ref<string | null>(null);

const isPhoneValid = computed(() => /^1\d{10}$/.test(phone.value));
const isCodeValid = computed(() => {
  const c = activationCode.value.trim().toUpperCase();
  // 支持带或不带 OC- 前缀，带或不带短横线
  return c.replace(/[-\s]/g, "").length >= 12;
});

let timer: ReturnType<typeof setInterval> | null = null;

async function sendSms() {
  if (!isPhoneValid.value) return;
  sending.value = true;
  error.value = null;
  try {
    await license.sendCode(phone.value);
    step.value = "sms";
    countdown.value = 60;
    timer = setInterval(() => {
      countdown.value--;
      if (countdown.value <= 0 && timer) {
        clearInterval(timer);
        timer = null;
      }
    }, 1000);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    sending.value = false;
  }
}

async function doPhoneLogin() {
  if (smsCode.value.length < 6) return;
  logging.value = true;
  error.value = null;
  try {
    await license.login(phone.value, smsCode.value);
    emit("success");
    emit("close");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    logging.value = false;
  }
}

async function doRedeem() {
  if (!isCodeValid.value) return;
  logging.value = true;
  error.value = null;
  try {
    await license.redeemCode(activationCode.value.trim().toUpperCase());
    emit("success");
    emit("close");
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    logging.value = false;
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
  width: 380px;
  display: flex; flex-direction: column; gap: 14px;
  box-shadow: 0 20px 60px rgba(0,0,0,.2);
}
h3 { font-size: 18px; font-weight: 700; margin: 0; }
.desc { font-size: 13px; color: var(--color-muted); margin: 0; }

/* 模式切换 Tab */
.mode-tabs {
  display: flex; gap: 0;
  border: 1px solid var(--color-border); border-radius: var(--radius);
  overflow: hidden;
}
.mode-tabs button {
  flex: 1; padding: 8px 0; font-size: 13px;
  border: none; background: var(--color-bg); cursor: pointer;
  color: var(--color-muted); transition: all .15s;
}
.mode-tabs button.active {
  background: var(--color-primary); color: #fff;
}
.mode-tabs button:not(.active):hover { background: var(--color-surface); }

.form-group { display: flex; flex-direction: column; gap: 4px; }
label { font-size: 12px; font-weight: 600; color: var(--color-muted); }
input {
  padding: 8px 12px; border: 1px solid var(--color-border);
  border-radius: var(--radius); font-size: 14px; width: 100%; box-sizing: border-box;
}
input:focus { outline: none; border-color: var(--color-primary); }
.code-input { font-family: monospace; letter-spacing: 2px; text-transform: uppercase; }
.code-row { display: flex; gap: 8px; }
.code-row input { flex: 1; }
.resend-btn { padding: 8px 12px; font-size: 12px; white-space: nowrap; }
.error-msg { color: var(--color-error); font-size: 12px; margin: 0; }
.btn-row { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }
</style>
