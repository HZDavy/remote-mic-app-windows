import type { KeyCode } from "./bridge";

/**
 * 手动录入快捷键（#215）。
 *
 * 部分按键在用户键盘上物理不存在（F13-F24），或需要以「通用修饰键」语义
 * 录入（`ctrl`、`win` 等），低级键盘钩子无法捕获它们。此模块把用户输入的
 * 键名文本解析为后端约定的 snake_case KeyCode 数组，供快捷键动作直接保存。
 */

/** 后端 `KeyCode` 枚举（snake_case）的全部合法值（键盘键，不含 chord 容器）。 */
const CANONICAL_KEYS: ReadonlySet<string> = new Set([
  "control",
  "left_control",
  "right_control",
  "shift",
  "left_shift",
  "right_shift",
  "alt",
  "left_alt",
  "right_alt",
  "left_windows",
  "right_windows",
  "backspace",
  "tab",
  "enter",
  "escape",
  "space",
  "page_up",
  "page_down",
  "end",
  "home",
  "left",
  "up",
  "right",
  "down",
  "insert",
  "delete",
  "apps",
  "volume_mute",
  "volume_down",
  "volume_up",
  "media_prev",
  "media_next",
  "media_play_pause",
  "oem3",
  "a",
  "b",
  "c",
  "d",
  "e",
  "f",
  "g",
  "h",
  "i",
  "j",
  "k",
  "l",
  "m",
  "n",
  "o",
  "p",
  "q",
  "r",
  "s",
  "t",
  "u",
  "v",
  "w",
  "x",
  "y",
  "z",
]);

/** 常见口语/厂商名 → 规范化 snake_case 键名。带侧别的修饰键优先，通用名映射到无侧别别名。 */
const ALIASES: ReadonlyMap<string, KeyCode> = new Map([
  // 通用修饰键（#215 显式要求 `ctrl` 可录入）。
  ["ctrl", "control"],
  ["control", "control"],
  ["shift", "shift"],
  ["alt", "alt"],
  ["opt", "alt"],
  ["option", "alt"],
  ["cmd", "left_windows"],
  ["command", "left_windows"],
  ["windows", "left_windows"],
  ["win", "left_windows"],
  ["super", "left_windows"],
  ["meta", "left_windows"],
  ["altgr", "right_alt"],
  // 侧别修饰键。
  ["leftctrl", "left_control"],
  ["lctrl", "left_control"],
  ["rightctrl", "right_control"],
  ["rctrl", "right_control"],
  ["leftcontrol", "left_control"],
  ["rightcontrol", "right_control"],
  ["leftshift", "left_shift"],
  ["lshift", "left_shift"],
  ["rightshift", "right_shift"],
  ["rshift", "right_shift"],
  ["leftalt", "left_alt"],
  ["lalt", "left_alt"],
  ["rightalt", "right_alt"],
  ["ralt", "right_alt"],
  ["leftwin", "left_windows"],
  ["lwin", "left_windows"],
  ["rightwin", "right_windows"],
  ["rwin", "right_windows"],
  // 常见对人的别名。
  ["esc", "escape"],
  ["return", "enter"],
  ["spacebar", "space"],
  ["spc", "space"],
  ["backspace", "backspace"],
  ["del", "delete"],
  ["ins", "insert"],
  ["pgup", "page_up"],
  ["pageup", "page_up"],
  ["pgdn", "page_down"],
  ["pagedown", "page_down"],
  ["右ctrl", "right_control"],
  ["左ctrl", "left_control"],
  ["右shift", "right_shift"],
  ["左shift", "left_shift"],
  ["右alt", "right_alt"],
  ["左alt", "left_alt"],
  ["win键", "left_windows"],
]);

/** 每个手动录入错误都带可展示的中文说明与坏 token。 */
export class ManualKeyError extends Error {
  readonly token: string;

  constructor(token: string) {
    super(`无法识别的键「${token}」，请输入英文键名（如 f13、ctrl、win）或把多键用 + 分隔`);
    this.token = token;
    this.name = "ManualKeyError";
  }
}

function canonicalize(token: string): KeyCode | null {
  const normalized = token.trim().toLocaleLowerCase();
  if (!normalized) return null;
  const alias = ALIASES.get(normalized);
  if (alias) return alias;
  if (CANONICAL_KEYS.has(normalized)) return normalized as KeyCode;
  // F13-F24：绝大多数键盘无实体，属本特性的主要目标。
  if (/^f(1[3-9]|2[0-4])$/.test(normalized)) return normalized as KeyCode;
  // 单数字键（0-9）→ digit0-9。
  if (/^[0-9]$/.test(normalized)) return `digit${normalized}` as KeyCode;
  // F1-F12 直接合法。
  if (/^f([1-9]|1[0-2])$/.test(normalized)) return normalized as KeyCode;
  return null;
}

/**
 * 把用户输入的键名文本解析为规范 KeyCode 数组。
 *
 * 支持用 `+`、`、`/`,` 或空白分隔多键；单键或误带空格也能识别。任一 token
 * 无法识别时抛出 {@link ManualKeyError}，避免把脏数据写进映射。
 */
export function parseManualChord(text: string): KeyCode[] {
  const tokens = text
    .split(/[+，,、\s]+/)
    .map((t) => t.trim())
    .filter(Boolean);
  if (tokens.length === 0) {
    throw new ManualKeyError(text);
  }
  return tokens.map((token) => {
    const key = canonicalize(token);
    if (!key) throw new ManualKeyError(token);
    return key;
  });
}

/** 是否为可被手动录入识别的键名（含别名与 F13-F24）。 */
export function isRecognizedManualKey(token: string): boolean {
  return canonicalize(token) !== null;
}