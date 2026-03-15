import Koa from "koa";
import bodyParser from "koa-bodyparser";
import Router from "koa-router";
import { initDb, getDb } from "./db.js";
import { authRoutes } from "./routes/auth.js";
import { skillsRoutes } from "./routes/skills.js";
import { paymentsRoutes } from "./routes/payments.js";
import { codesRoutes } from "./routes/codes.js";
import { activationRoutes } from "./routes/activation.js";
import { ratelimit } from "./middleware/ratelimit.js";

const app = new Koa();
const router = new Router({ prefix: "/api" });

// 初始化数据库
initDb();

// 中间件
app.use(ratelimit());
app.use(bodyParser({ enableTypes: ["json", "text"], extendTypes: { text: ["text/xml", "application/xml"] } }));

// 错误处理
app.use(async (ctx, next) => {
  try {
    await next();
  } catch (err) {
    ctx.status = err.status || 500;
    ctx.body = { error: err.message };
    console.error(`[${new Date().toISOString()}] ${ctx.method} ${ctx.path} → ${ctx.status}: ${err.message}`);
  }
});

// 管理操作审计中间件
app.use(async (ctx, next) => {
  await next();
  if (ctx.path.startsWith("/api/admin") && ctx.method !== "GET" && ctx.status < 400) {
    try {
      const db = getDb();
      db.prepare(
        "INSERT INTO admin_audit_log (action, detail, ip) VALUES (?, ?, ?)"
      ).run(
        `${ctx.method} ${ctx.path}`,
        JSON.stringify(ctx.request.body || {}).slice(0, 500),
        ctx.ip
      );
    } catch { /* 审计日志写入失败不影响主流程 */ }
  }
});

// 注册路由
authRoutes(router);
skillsRoutes(router);
paymentsRoutes(router);
codesRoutes(router);
activationRoutes(router);

app.use(router.routes()).use(router.allowedMethods());

const PORT = process.env.PORT || 3800;
app.listen(PORT, () => {
  console.log(`[license-server] 启动在 http://0.0.0.0:${PORT}`);
});
