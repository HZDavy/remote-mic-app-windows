import { describe, expect, it } from "vitest";
import {
  isRecognizedManualKey,
  ManualKeyError,
  parseManualChord,
} from "./manual-keys";

describe("手动录入 parseManualChord（#215）", () => {
  it("识别 F13-F24（键盘常无实体、无法录制的目标键）", () => {
    expect(parseManualChord("f13")).toEqual(["f13"]);
    expect(parseManualChord("F24")).toEqual(["f24"]);
    expect(parseManualChord("f20")).toEqual(["f20"]);
    expect(parseManualChord("f13", )).toEqual(["f13"]);
  });

  it("把通用修饰键别名归一化为 snake_case", () => {
    expect(parseManualChord("ctrl")).toEqual(["control"]);
    expect(parseManualChord("ctrl + f13")).toEqual(["control", "f13"]);
    expect(parseManualChord("win+d")).toEqual(["left_windows", "d"]);
    expect(parseManualChord("option, f5")).toEqual(["alt", "f5"]);
    expect(parseManualChord("altgr")).toEqual(["right_alt"]);
    expect(parseManualChord("esc")).toEqual(["escape"]);
  });

  it("支持中文分隔符、单数字键与大小写混写", () => {
    expect(parseManualChord("ctrl、shift、enter")).toEqual([
      "control",
      "shift",
      "enter",
    ]);
    expect(parseManualChord("ctrl 5")).toEqual(["control", "digit5"]);
    expect(parseManualChord("  ctrl  +  f17 ")).toEqual(["control", "f17"]);
  });

  it("未识别 token 抛 ManualKeyError 且写明坏键", () => {
    for (const bad of ["注意", "f2x", "control+crazykey"]) {
      expect(() => parseManualChord(bad)).toThrow(ManualKeyError);
    }
    expect(() => parseManualChord("ctrl + 不存在")).toThrow(/不存在/);
  });

  it("空输入抛错", () => {
    expect(() => parseManualChord("   ")).toThrow(ManualKeyError);
    expect(() => parseManualChord("")).toThrow(ManualKeyError);
  });

  it("isRecognizedManualKey 只认已知键与别名", () => {
    expect(isRecognizedManualKey("f13")).toBe(true);
    expect(isRecognizedManualKey("ctrl")).toBe(true);
    expect(isRecognizedManualKey("win")).toBe(true);
    expect(isRecognizedManualKey("garbage_key_")).toBe(false);
  });
});