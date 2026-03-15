/**
 * 零宽字符水印编码/解码
 *
 * 将 userId 编码为零宽字符序列，插入到内容中，
 * 用于追踪付费内容泄漏来源。
 */

const ZWCHARS = ["\u200B", "\u200C", "\u200D", "\uFEFF"];

/**
 * 将 userId 编码为零宽字符并嵌入内容
 * @param {string} content - 原始内容
 * @param {string} userId - 用户 ID
 * @returns {string} 带水印的内容
 */
export function embedWatermark(content, userId) {
  const bits = Buffer.from(userId, "utf-8")
    .toString("binary")
    .split("")
    .map((c) => c.charCodeAt(0).toString(2).padStart(8, "0"))
    .join("");

  // 每 2 位编码为一个零宽字符（4 种字符 = 2 bits）
  const wm = [];
  for (let i = 0; i < bits.length; i += 2) {
    const idx = parseInt(bits.slice(i, i + 2), 2);
    wm.push(ZWCHARS[idx]);
  }
  const wmStr = wm.join("");

  // 插入到第一个空行之后（Markdown 段落分隔处）
  const insertIdx = content.indexOf("\n\n");
  if (insertIdx !== -1) {
    return content.slice(0, insertIdx + 1) + wmStr + content.slice(insertIdx + 1);
  }
  // 无空行时追加到末尾
  return content + wmStr;
}

/**
 * 从内容中提取水印并还原 userId
 * @param {string} content - 带水印的内容
 * @returns {string|null} 提取的 userId，无水印时返回 null
 */
export function extractWatermark(content) {
  // 提取所有零宽字符
  const zwRegex = /[\u200B\u200C\u200D\uFEFF]+/g;
  const matches = content.match(zwRegex);
  if (!matches) return null;

  // 取最长的连续零宽序列（即水印）
  const wmStr = matches.reduce((a, b) => (a.length > b.length ? a : b));
  if (wmStr.length < 4) return null;

  // 解码：每个零宽字符 → 2 bits
  let bits = "";
  for (const ch of wmStr) {
    const idx = ZWCHARS.indexOf(ch);
    if (idx === -1) return null;
    bits += idx.toString(2).padStart(2, "0");
  }

  // 按 8 位分组还原字节
  const bytes = [];
  for (let i = 0; i + 8 <= bits.length; i += 8) {
    bytes.push(parseInt(bits.slice(i, i + 8), 2));
  }

  try {
    return Buffer.from(bytes).toString("utf-8");
  } catch {
    return null;
  }
}
