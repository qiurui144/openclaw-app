<template>
  <Teleport to="body">
    <div class="manual-overlay" v-if="modelValue" @click.self="$emit('update:modelValue', false)">
      <div class="manual-modal">
        <div class="modal-header">
          <h2>📖 OpenClaw 操作手册</h2>
          <button class="close-btn" @click="$emit('update:modelValue', false)">✕</button>
        </div>

        <div class="modal-body">
          <nav class="toc">
            <button
              v-for="sec in sections"
              :key="sec.id"
              class="toc-item"
              :class="{ active: activeSection === sec.id }"
              @click="activeSection = sec.id"
            >{{ sec.label }}</button>
          </nav>

          <div class="content">
            <!-- 简介 -->
            <section v-show="activeSection === 'intro'">
              <h3>什么是 OpenClaw？</h3>
              <p>OpenClaw 是一款<strong>企业级多平台机器人网关</strong>，支持将 AI 能力（Skills）接入企业微信、钉钉、飞书、QQ 频道等平台。</p>
              <h4>核心功能</h4>
              <ul>
                <li><strong>多平台接入</strong>：一套服务，同时服务多个企业 IM 平台</li>
                <li><strong>Skills 扩展</strong>：通过插件扩展机器人能力（问答、工单、知识库等）</li>
                <li><strong>网页控制台</strong>：浏览器管理机器人规则、用户权限和运行日志</li>
                <li><strong>自动更新</strong>：Skills 支持热更新，无需重启服务</li>
              </ul>
              <h4>系统要求</h4>
              <ul>
                <li>Linux（内核 ≥ 5.0）或 Windows 10（Build 17134+）</li>
                <li>可用磁盘空间 ≥ 512 MB</li>
                <li>端口 18789 未被占用（可修改）</li>
              </ul>
            </section>

            <!-- 安装步骤 -->
            <section v-show="activeSection === 'steps'">
              <h3>安装步骤说明</h3>

              <div class="step-block">
                <div class="step-num">1</div>
                <div>
                  <strong>欢迎</strong>
                  <p>了解 OpenClaw 功能特性。若检测到已有安装，可选择升级或全新安装。</p>
                </div>
              </div>

              <div class="step-block">
                <div class="step-num">2</div>
                <div>
                  <strong>环境检查</strong>
                  <p>自动检测操作系统版本、磁盘空间、端口占用和运行权限，确保系统满足安装要求。</p>
                </div>
              </div>

              <div class="step-block">
                <div class="step-num">3</div>
                <div>
                  <strong>安装来源</strong>
                  <p>选择安装包来源：<br>
                  • <strong>Full Bundle</strong>（推荐）：已离线打包所有依赖，无需联网<br>
                  • <strong>在线下载</strong>：需要访问 npm/GitHub 下载资源<br>
                  • <strong>本地压缩包</strong>：使用您已下载的 openclaw.tgz</p>
                </div>
              </div>

              <div class="step-block">
                <div class="step-num">4</div>
                <div>
                  <strong>安装路径</strong>
                  <p>选择 OpenClaw 的安装目录和启动方式。建议启用系统服务和开机自启。</p>
                </div>
              </div>

              <div class="step-block">
                <div class="step-num">5</div>
                <div>
                  <strong>服务配置</strong>
                  <p>设置控制台端口（默认 18789）、可选域名和管理员密码。密码用于登录网页控制台。</p>
                </div>
              </div>

              <div class="step-block">
                <div class="step-num">6</div>
                <div>
                  <strong>平台集成</strong>
                  <p>可选步骤：配置企业 IM 平台（企业微信/钉钉/飞书/QQ）的机器人 Webhook 地址。安装后也可在控制台添加。</p>
                </div>
              </div>

              <div class="step-block">
                <div class="step-num">7</div>
                <div>
                  <strong>部署</strong>
                  <p>自动执行：解压资源 → 配置文件 → 安装服务 → 健康检查。可在日志面板看到详细进度。</p>
                </div>
              </div>

              <div class="step-block">
                <div class="step-num">8</div>
                <div>
                  <strong>完成</strong>
                  <p>安装成功后打开网页控制台（默认 http://127.0.0.1:18789）开始使用。</p>
                </div>
              </div>
            </section>

            <!-- 权限说明 -->
            <section v-show="activeSection === 'permissions'">
              <h3>权限与安装模式</h3>

              <h4>普通用户运行（推荐用于个人使用）</h4>
              <ul>
                <li>默认安装目录：<code>~/openclaw</code>（Linux）或 <code>%LOCALAPPDATA%\openclaw</code>（Windows）</li>
                <li>注册用户级 systemd 服务（<code>systemctl --user</code>）</li>
                <li>只对当前用户生效，重启后需要用户登录才会自启</li>
              </ul>

              <h4>root/管理员运行（推荐用于服务器部署）</h4>
              <ul>
                <li>默认安装目录：<code>/opt/openclaw</code>（Linux）或 <code>C:\Program Files\openclaw</code>（Windows）</li>
                <li>注册系统级 systemd 服务（<code>systemctl</code>，无 --user）</li>
                <li>系统启动时自动运行，无需用户登录</li>
                <li>Linux：使用 <code>sudo ./OpenClaw</code> 或 <code>sudo -E ./OpenClaw</code> 启动向导</li>
                <li>Windows：右键「以管理员身份运行」</li>
              </ul>

              <div class="tip-box">
                💡 建议服务器部署使用 root/管理员模式，保证开机自启可靠性。个人电脑使用普通用户模式即可。
              </div>
            </section>

            <!-- 平台集成 -->
            <section v-show="activeSection === 'platform'">
              <h3>平台集成指南</h3>
              <p>Webhook 是平台将消息推送到您服务的地址。OpenClaw 通过 Webhook 接收来自各平台的消息，并将机器人回复发回群组。</p>

              <h4>💼 企业微信</h4>
              <ol>
                <li>打开企业微信群 → 右上角「…」→「群机器人」→「添加机器人」</li>
                <li>创建机器人后复制 Webhook 地址（格式：<code>https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx</code>）</li>
                <li>参考文档：<a href="javascript:void(0)">企业微信官方文档</a></li>
              </ol>

              <h4>⚙️ 钉钉</h4>
              <ol>
                <li>打开钉钉群 → 右上角「…」→「智能群助手」→「添加机器人」→「自定义」</li>
                <li>设置安全方式（推荐「加签」），复制 Webhook 地址（格式：<code>https://oapi.dingtalk.com/robot/send?access_token=xxx</code>）</li>
                <li>注意：钉钉机器人有 20条/分钟 的频率限制</li>
              </ol>

              <h4>🪁 飞书</h4>
              <ol>
                <li>打开飞书群 → 右上角「…」→「设置」→「群机器人」→「添加机器人」→「自定义机器人」</li>
                <li>复制 Webhook 地址（格式：<code>https://open.feishu.cn/open-apis/bot/v2/hook/xxx</code>）</li>
              </ol>

              <h4>🐧 QQ 频道</h4>
              <ol>
                <li>访问 QQ 机器人开放平台创建机器人应用</li>
                <li>在「Webhook 配置」中获取接收消息的 URL</li>
                <li>文档：<a href="javascript:void(0)">QQ 机器人开放平台</a></li>
              </ol>

              <div class="tip-box">
                💡 可跳过此步骤，安装完成后在网页控制台 → 「平台管理」中随时添加或修改。
              </div>
            </section>

            <!-- 常见问题 -->
            <section v-show="activeSection === 'faq'">
              <h3>常见问题</h3>

              <div class="faq-item">
                <div class="faq-q">Q：部署卡在某一步没有进展？</div>
                <div class="faq-a">展开底部日志面板查看详细错误信息。常见原因：网络超时（切换到 Full Bundle 版）、端口被占用（修改端口）、权限不足（以 root/管理员重新运行）。</div>
              </div>

              <div class="faq-item">
                <div class="faq-q">Q：安装后如何手动启动/停止服务？</div>
                <div class="faq-a">
                  Linux（root）：<code>systemctl start/stop openclaw</code><br>
                  Linux（用户）：<code>systemctl --user start/stop openclaw</code><br>
                  Windows：在任务计划程序中找到 openclaw 任务<br>
                  通用：<code>~/.openclaw/uninstall.sh</code>（Linux）或 <code>%APPDATA%\openclaw\uninstall.bat</code>（Windows）
                </div>
              </div>

              <div class="faq-item">
                <div class="faq-q">Q：如何更新 OpenClaw 版本？</div>
                <div class="faq-a">在完成页面或网页控制台「系统设置」中点击「检查更新」，自动下载并应用最新版本，支持失败回滚。</div>
              </div>

              <div class="faq-item">
                <div class="faq-q">Q：忘记管理员密码怎么办？</div>
                <div class="faq-a">编辑 <code>~/.openclaw/openclaw.json</code>，找到 <code>admin_password</code> 字段修改后重启服务。</div>
              </div>

              <div class="faq-item">
                <div class="faq-q">Q：如何卸载 OpenClaw？</div>
                <div class="faq-a">执行安装目录下的 <code>uninstall.sh</code>（Linux）或 <code>uninstall.bat</code>（Windows），脚本将停止服务并删除所有文件。</div>
              </div>

              <div class="faq-item">
                <div class="faq-q">Q：Full Bundle 和 Lite 版本有什么区别？</div>
                <div class="faq-a">Full Bundle（~169MB）：内置 Node.js、openclaw 服务包和 Mihomo，完全离线安装。Lite 版（~76MB）：安装时需要联网下载依赖，适合网络良好环境。</div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from "vue";

defineProps<{ modelValue: boolean }>();
defineEmits<{ "update:modelValue": [boolean] }>();

const activeSection = ref("intro");

const sections = [
  { id: "intro", label: "简介" },
  { id: "steps", label: "步骤说明" },
  { id: "permissions", label: "权限说明" },
  { id: "platform", label: "平台集成" },
  { id: "faq", label: "常见问题" },
];
</script>

<style scoped>
.manual-overlay {
  position: fixed; inset: 0; z-index: 1000;
  background: rgba(0,0,0,.5); display: flex;
  align-items: center; justify-content: center;
}

.manual-modal {
  background: var(--color-bg); border-radius: var(--radius);
  width: 680px; max-width: 95vw; max-height: 80vh;
  display: flex; flex-direction: column;
  box-shadow: 0 20px 60px rgba(0,0,0,.3);
}

.modal-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 16px 20px; border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}
.modal-header h2 { font-size: 18px; font-weight: 700; }
.close-btn {
  background: none; border: none; font-size: 16px;
  color: var(--color-muted); cursor: pointer; padding: 4px 8px;
  border-radius: 4px;
}
.close-btn:hover { background: var(--color-surface); }

.modal-body { display: flex; flex: 1; overflow: hidden; }

.toc {
  width: 120px; flex-shrink: 0;
  border-right: 1px solid var(--color-border);
  display: flex; flex-direction: column;
  padding: 12px 0;
  background: var(--color-surface);
}
.toc-item {
  text-align: left; background: none; border: none;
  padding: 10px 16px; font-size: 13px; cursor: pointer;
  color: var(--color-muted); transition: all .15s;
}
.toc-item:hover { background: var(--color-border); color: var(--color-text); }
.toc-item.active { background: var(--color-bg); color: var(--color-primary); font-weight: 600; border-right: 3px solid var(--color-primary); }

.content {
  flex: 1; overflow-y: auto; padding: 20px 24px;
}
.content h3 { font-size: 16px; font-weight: 700; margin-bottom: 14px; }
.content h4 { font-size: 14px; font-weight: 600; margin: 16px 0 8px; }
.content p { font-size: 13px; line-height: 1.8; color: #475569; margin-bottom: 8px; }
.content ul, .content ol { padding-left: 20px; font-size: 13px; line-height: 1.9; color: #475569; }
.content li { margin-bottom: 4px; }
.content code {
  background: #f1f5f9; border-radius: 3px;
  padding: 1px 5px; font-family: monospace; font-size: 12px;
}

.step-block {
  display: flex; gap: 12px; align-items: flex-start;
  margin-bottom: 16px; padding-bottom: 16px;
  border-bottom: 1px solid var(--color-border);
}
.step-block:last-child { border-bottom: none; }
.step-num {
  width: 24px; height: 24px; border-radius: 50%;
  background: var(--color-primary); color: white;
  display: flex; align-items: center; justify-content: center;
  font-size: 12px; font-weight: 700; flex-shrink: 0; margin-top: 2px;
}

.faq-item { margin-bottom: 16px; padding-bottom: 16px; border-bottom: 1px solid var(--color-border); }
.faq-item:last-child { border-bottom: none; }
.faq-q { font-weight: 600; font-size: 13px; margin-bottom: 6px; }
.faq-a { font-size: 13px; line-height: 1.8; color: #475569; }

.tip-box {
  background: #eff6ff; border-left: 3px solid #3b82f6;
  padding: 10px 14px; border-radius: 0 4px 4px 0;
  font-size: 13px; margin-top: 16px; line-height: 1.7;
}
</style>
