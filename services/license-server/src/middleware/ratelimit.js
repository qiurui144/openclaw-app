/**
 * 基于内存的滑动窗口 IP 限流中间件
 *
 * 策略：
 * - 全局：单 IP 每分钟 60 请求
 * - /admin/*：已有 X-Admin-Key 保护，不额外限流
 */

const windows = new Map();

const CLEANUP_INTERVAL = 60_000;
let lastCleanup = Date.now();

function isRateLimited(key, limit, windowMs) {
  const now = Date.now();
  const windowStart = now - windowMs;

  // 惰性清理（每分钟一次）
  if (now - lastCleanup > CLEANUP_INTERVAL) {
    lastCleanup = now;
    for (const [k, timestamps] of windows) {
      const filtered = timestamps.filter((t) => t > windowStart);
      if (filtered.length === 0) {
        windows.delete(k);
      } else {
        windows.set(k, filtered);
      }
    }
  }

  const timestamps = windows.get(key) || [];
  const recent = timestamps.filter((t) => t > windowStart);

  if (recent.length >= limit) {
    windows.set(key, recent);
    return true;
  }

  recent.push(now);
  windows.set(key, recent);
  return false;
}

/**
 * Koa 限流中间件
 */
export function ratelimit() {
  return async (ctx, next) => {
    const ip = ctx.ip || ctx.request.ip || "unknown";

    // /admin/* 不限流（已有 X-Admin-Key 保护）
    if (ctx.path.startsWith("/api/admin")) {
      await next();
      return;
    }

    // 全局限流：单 IP 每分钟 60 次
    if (isRateLimited(`global:${ip}`, 60, 60_000)) {
      ctx.status = 429;
      ctx.body = { error: "请求过于频繁，请稍后再试" };
      return;
    }

    await next();
  };
}
