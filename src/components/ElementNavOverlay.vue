<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  navMove,
  navScan,
  type NavDir,
  type NavElement,
  type NavMoveOutcome,
  type NormalizedRect,
} from "../lib/bridge";

/**
 * 通用屏幕元素导航（#214）：在既有 UIA 基座上扩展的方向键遍历。
 *
 * 行为口径：
 * - 打开时扫描前台窗口的候选元素（navScan），把第一个「应被 Invoke/聚焦」元素作为起点；
 * - 方向键/OK 在导航模式下**不再注入原生键**，而是沿 UIA 遍历（navMove），
 *   并把命中元素框画在「模拟屏幕」预览上供肉眼确认；
 * - OK 命中可触发元素时，交由后端 Invoke/聚焦（navMove 的 activated 字段反馈结果）；
 * - 浏览器预览下为只读演示（navScan 返回空），组件仍可交互但不产生副作用。
 *
 * 纯展示 + 转发组件：不发后端开关命令，导航模式开关由父页面决定并传入 `active`，
 * 本组件只负责扫描/移动/画框。
 */
const props = defineProps<{
  active: boolean;
}>();
const emit = defineEmits<{ exit: []; fire: [dir: NavDir] }>();

const scanning = ref(false);
const moving = ref(false);
const error = ref("");
const lastOutcome = ref<NavMoveOutcome | null>(null);
const elements = ref<NavElement[]>([]);
const current = ref<NormalizedRect | null>(null);

const hasElements = computed(() => elements.value.length > 0);
const outcomeText = computed(() => {
  const o = lastOutcome.value;
  if (!o) return "";
  switch (o.type) {
    case "moved":
      return o.activated ? `已激活 #${o.index}` : `已框选 #${o.index}`;
    case "no_element":
      return "该方向没有可导航元素";
    case "target_exited":
      return "目标窗口已退出";
    case "failed":
      return `导航失败：${o.reason}`;
    case "unavailable":
      return "当前平台不支持元素导航";
  }
});

/** 画面固定 1000x600 的「模拟屏幕」，矩形一律按该尺寸归一化。 */
const VIEW_W = 1000;
const VIEW_H = 600;
const screenStyle = computed(() => {
  if (!current.value) return {};
  const r = current.value;
  return {
    left: `${Math.max(0, Math.min(VIEW_W - 2, r.x * VIEW_W))}px`,
    top: `${Math.max(0, Math.min(VIEW_H - 2, r.y * VIEW_H))}px`,
    width: `${Math.max(0, Math.min(VIEW_W, r.width * VIEW_W))}px`,
    height: `${Math.max(0, Math.min(VIEW_H, r.height * VIEW_H))}px`,
  };
});

async function refresh(): Promise<void> {
  if (!props.active) return;
  scanning.value = true;
  error.value = "";
  try {
    const list = await navScan();
    elements.value = list ?? [];
    // 起点取第一个应被 Invoke/聚焦的元素；否则取首个候选。
    current.value =
      list.find((el) => !preferInvoke(el))?.rect ??
      list[0]?.rect ??
      null;
    lastOutcome.value = null;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    scanning.value = false;
  }
}

function preferInvoke(el: NavElement): boolean {
  return ["button", "list_item", "link", "menu_item", "check_box", "radio_button", "tab_item"].includes(
    el.etype,
  );
}

function move(dir: NavDir): void {
  if (!props.active || moving.value) return;
  const cursor = current.value;
  if (!cursor) {
    lastOutcome.value = { type: "no_element" };
    return;
  }
  moving.value = true;
  error.value = "";
  void navMove(cursor, dir)
    .then((outcome) => {
      lastOutcome.value = outcome;
      if (outcome?.type === "moved") {
        current.value = outcome.rect;
      }
      emit("fire", dir);
    })
    .catch((e) => {
      error.value = e instanceof Error ? e.message : String(e);
    })
    .finally(() => {
      moving.value = false;
    });
}

function onKeydown(event: KeyboardEvent): void {
  if (!props.active) return;
  const dir: NavDir | undefined =
    event.key === "ArrowUp"
      ? "up"
      : event.key === "ArrowDown"
        ? "down"
        : event.key === "ArrowLeft"
          ? "left"
          : event.key === "ArrowRight"
            ? "right"
            : undefined;
  if (!dir) return;
  event.preventDefault();
  event.stopPropagation();
  move(dir);
}

watch(
  () => props.active,
  (on) => {
    if (on) {
      void refresh();
      window.addEventListener("keydown", onKeydown, true);
    } else {
      window.removeEventListener("keydown", onKeydown, true);
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <div class="element-nav" role="dialog" aria-label="元素导航">
    <div class="nav-toolbar">
      <span class="nav-title">元素导航</span>
      <span v-if="scanning" class="nav-status">扫描中…</span>
      <span v-else-if="hasElements" class="nav-status">已发现 {{ elements.length }} 个元素</span>
      <span v-else class="nav-status">未发现可导航元素（仅预览演示）</span>
      <button type="button" class="nav-action" :disabled="scanning" @click="refresh">重新扫描</button>
      <button type="button" class="nav-action" :disabled="moving" @click="emit('exit')">退出导航</button>
    </div>

    <div class="nav-screen" :class="{ empty: !hasElements }">
      <div
        v-if="current"
        class="nav-highlight"
        :style="screenStyle"
      ></div>
      <span v-if="!hasElements && !scanning" class="screen-placeholder">
        未发现候选元素。请把目标应用切到前台后重试。
      </span>
    </div>

    <div class="nav-pad">
      <div class="pad-row">
        <button type="button" class="pad-key pad-up" :disabled="moving" aria-label="向上一格" title="向上一格"
          @click="move('up')">↑</button>
      </div>
      <div class="pad-row">
        <button type="button" class="pad-key pad-left" :disabled="moving" aria-label="向左一格" title="向左一格"
          @click="move('left')">←</button>
        <button type="button" class="pad-key pad-go" :disabled="moving" aria-label="重新扫描" title="重新扫描候选元素"
          @click="refresh">刷新</button>
        <button type="button" class="pad-key pad-right" :disabled="moving" aria-label="向右一格" title="向右一格"
          @click="move('right')">→</button>
      </div>
      <div class="pad-row">
        <button type="button" class="pad-key pad-down" :disabled="moving" aria-label="向下一格" title="向下一格"
          @click="move('down')">↓</button>
      </div>
    </div>

    <p v-if="outcomeText" class="nav-outcome" aria-live="polite">{{ outcomeText }}</p>
    <p v-if="error" class="nav-error" role="alert">{{ error }}</p>
  </div>
</template>

<style scoped>
.element-nav {
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-sizing: border-box;
  padding: 14px;
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  background: var(--surface-canvas);
  color: var(--text-primary);
}
.nav-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}
.nav-title { font-weight: 600; }
.nav-status { font-size: 12px; opacity: 0.8; }
.nav-action {
  margin-left: auto;
  padding: 5px 10px;
  font: inherit;
  font-size: 12.5px;
  color: inherit;
  background: transparent;
  border: 1px solid currentColor;
  border-radius: 5px;
  cursor: pointer;
}
.nav-action:disabled { opacity: 0.5; cursor: not-allowed; }
.nav-screen {
  position: relative;
  width: 100%;
  aspect-ratio: 1000 / 600;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background:
    linear-gradient(#00000022, #00000022),
    repeating-linear-gradient(0deg, transparent, transparent 47px, #00000011 47px, #00000011 48px),
    repeating-linear-gradient(90deg, transparent, transparent 79px, #00000011 79px, #00000011 80px);
  overflow: hidden;
}
.screen-placeholder {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  font-size: 13px;
  opacity: 0.75;
  text-align: center;
  padding: 12px;
}
.nav-highlight {
  position: absolute;
  box-sizing: border-box;
  border: 2px solid var(--accent-text, #2f7bff);
  border-radius: 3px;
  background: color-mix(in srgb, var(--accent-text, #2f7bff) 22%, transparent);
  box-shadow: 0 0 0 1px #00000066;
  transition: left 0.08s ease, top 0.08s ease, width 0.08s ease, height 0.08s ease;
}
.nav-pad { display: grid; grid-template-columns: 1fr auto 1fr; gap: 8px; align-items: center; }
.pad-row { display: flex; justify-content: center; gap: 8px; }
.pad-key {
  min-width: 46px;
  height: 42px;
  padding: 0 8px;
  font-size: 18px;
  color: inherit;
  background: transparent;
  border: 1px solid currentColor;
  border-radius: 6px;
  cursor: pointer;
}
.pad-key:disabled { opacity: 0.5; cursor: not-allowed; }
.pad-go { font-size: 13px; }
.nav-outcome, .nav-error { margin: 0; font-size: 12.5px; }
.nav-error { color: var(--error-color, #d64545); }
</style>