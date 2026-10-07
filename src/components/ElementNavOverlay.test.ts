// @vitest-environment jsdom

import { mount, type VueWrapper } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";
import ElementNavOverlay from "./ElementNavOverlay.vue";

vi.mock("../lib/bridge", () => ({
  navScan: vi.fn(async () => [
    { rect: { x: 0.1, y: 0.2, width: 0.2, height: 0.1 }, etype: "button" },
    { rect: { x: 0.4, y: 0.4, width: 0.2, height: 0.1 }, etype: "list_item" },
  ]),
  navMove: vi.fn(async () => ({
    type: "moved",
    index: 1,
    activated: true,
    rect: { x: 0.4, y: 0.4, width: 0.2, height: 0.1 },
  })),
}));

let wrapper: VueWrapper | undefined;

beforeEach(() => {
  vi.clearAllMocks();
});

async function mountActive(): Promise<VueWrapper> {
  wrapper = mount(ElementNavOverlay, { props: { active: true } });
  await nextTick();
  return wrapper;
}

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
});

describe("element nav overlay", () => {
  it("激活时扫描前沿元素并画首框", async () => {
    const wrap = await mountActive();
    expect(wrap.find(".nav-highlight").exists()).toBe(true);
    expect(wrap.text()).toContain("已发现 2 个元素");
  });

  it("键盘方向键在激活时触发 navMove 并沿结果移动高亮框", async () => {
    const wrap = await mountActive();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", cancelable: true }));
    await nextTick();
    const { navMove } = await import("../lib/bridge");
    expect(navMove).toHaveBeenCalledOnce();
    expect((navMove as ReturnType<typeof vi.fn>).mock.calls[0][1]).toBe("right");
    const box = wrap.find(".nav-highlight").attributes("style");
    expect(box).toContain("left: 400px");
  });

  it("未激活时不挂方向键路由", async () => {
    wrapper = mount(ElementNavOverlay, { props: { active: false } });
    await nextTick();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", cancelable: true }));
    await nextTick();
    const { navMove } = await import("../lib/bridge");
    expect(navMove).not.toHaveBeenCalled();
  });
});