//! 通用屏幕元素导航（#214）：在既有 UIA（聚焦输入框）基座上扩展元素级导航。
//!
//! 本模块只承载**纯逻辑**（跨平台、可单测），不触 Windows：
//!
//! - [`NavElement`]：一个可交互 UIA 元素的归一化矩形 + 元素类型。
//! - [`pick_at`]：按方向 + 最近邻做空间选点，返回被选中元素的序号。
//! - [`prefer_invoke`]：某元素类型是否应优先走 UIA `Invoke`（否则回退 `SetFocus`）。
//!
//! UIA 侧（`nav_windows`）负责：放宽候选门槛到可交互元素、按方向驱动
//! `pick_at`、执行 `Invoke`/`SetFocus` 并用 `CompareRuntimeIds` 读回确认。
//!
//! 边界（与 issue 一致）：只用公开辅助功能接口，不读改第三方私有状态；
//! 无 UIA 树时上层返回「无元素」而不是退化坐标点按；不做进程注入。

use serde::{Deserialize, Serialize};

/// 导航方向（与遥控器 D-pad 对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavDir {
    Up,
    Down,
    Left,
    Right,
}

impl NavDir {
    /// 该方向上，元素中心相对游标中心的带符号主位移（严格朝该方向为正）。
    fn signed_primary(self, candidate: &NavElement, cx: f64, cy: f64) -> f64 {
        let (ex, ey) = candidate.center_xy();
        match self {
            Self::Up => cy - ey,
            Self::Down => ey - cy,
            Self::Left => cx - ex,
            Self::Right => ex - cx,
        }
    }

    /// 该方向的正交偏移量（越小越对齐，用于主位移相近时破平局）。
    fn orthogonal(self, candidate: &NavElement, cx: f64, cy: f64) -> f64 {
        let (ex, ey) = candidate.center_xy();
        match self {
            Self::Up | Self::Down => (ex - cx).abs(),
            Self::Left | Self::Right => (ey - cy).abs(),
        }
    }
}

/// 元素的归一化矩形（复用聚焦模块的类型，保证坐标系一致）。
pub use crate::focus::NormalizedRect;

/// 可交互元素类型分类（UIA `ControlType` 放宽后的粗分）。
///
/// 只区分我们是否应优先 `Invoke`；不做更细的语义推断。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavElementType {
    /// 按钮：明确 `Invoke` 优先。
    Button,
    /// 列表项：`SelectionItem`/`Invoke` 均可，优先 `Invoke`。
    ListItem,
    /// 链接：`Invoke` 优先。
    Link,
    /// 菜单项：`Invoke` 优先。
    MenuItem,
    /// 复选/单选：`Toggle`/`SelectionItem`，`Invoke` 通常可用。
    CheckBox,
    RadioButton,
    /// 树项：`SelectionItem`，`Invoke` 有时不可用，走 `SetFocus` 兜底。
    TreeItem,
    /// 标签/页签：`SelectionItem` 优先。
    TabItem,
    /// 其余可交互元素（无法确认 invoke 能力）：仅 `SetFocus`。
    Other,
}

impl NavElementType {
    /// 该类型是否应优先走 UIA `Invoke`。
    pub fn prefer_invoke(self) -> bool {
        matches!(
            self,
            Self::Button
                | Self::ListItem
                | Self::Link
                | Self::MenuItem
                | Self::CheckBox
                | Self::RadioButton
                | Self::TabItem
        )
    }
}

/// 一个可交互导航元素。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavElement {
    /// 矩形（归一化坐标，与 `NormalizedRect::from_bounds` 对齐）。
    pub rect: NormalizedRect,
    /// 元素类型（决定 `Invoke`/`SetFocus` 优先级）。
    pub etype: NavElementType,
}

impl NavElement {
    fn center_x(&self) -> f64 {
        self.rect.x + self.rect.width / 2.0
    }
    fn center_y(&self) -> f64 {
        self.rect.y + self.rect.height / 2.0
    }
    fn center_xy(&self) -> (f64, f64) {
        (self.center_x(), self.center_y())
    }
}

fn center(rect: &NormalizedRect) -> (f64, f64) {
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

/// 按方向 + 最近邻选点：从 `elems` 中选出沿 `dir` 方向、距离 `cursor` 最近的元素
/// 序号；无可选元素时返回 `None`。
///
/// 选点规则（与 issue 示意一致：按方向过滤 + `max_by_key` 打分）：
/// 1. 仅候选中心位于游标中心的主方向一侧（严格同侧，防止往回选）。
/// 2. 主方向位移最大者优先（正向平移越远分值越高——对号排序）。
/// 3. 主位移差异极小（近同排）时，用正交偏移做次优排序（越对齐越好）。
pub fn pick_at(cursor: &NormalizedRect, dir: NavDir, elems: &[NavElement]) -> Option<usize> {
    let (cx, cy) = center(cursor);
    let mut best: Option<(usize, f64, f64)> = None; // (idx, primary, orthogonal)
    for (idx, elem) in elems.iter().enumerate() {
        if elem.rect.width <= 0.0 || elem.rect.height <= 0.0 {
            continue;
        }
        let primary = dir.signed_primary(elem, cx, cy);
        if primary <= 0.0 {
            // 不在该方向一侧（严格朝该方向才入选，防止往回选）。
            continue;
        }
        let orthogonal = dir.orthogonal(elem, cx, cy);
        let better = match best {
            None => true,
            Some((_, best_primary, best_orthogonal)) => {
                // 主位移越小（越近）越好；主位移相近时，正交偏移越小（越对齐）越好。
                const EPS: f64 = 1e-6;
                if (primary - best_primary).abs() > EPS {
                    primary < best_primary
                } else {
                    orthogonal < best_orthogonal
                }
            }
        };
        if better {
            best = Some((idx, primary, orthogonal));
        }
    }
    best.map(|(idx, _, _)| idx)
}

/// 一次「向某方向移动并激活」的跨平台结果（对 Tauri / 前端契约）。
///
/// `focus_windows::NavMoveResult` 是 Windows 内部形态；这里统一为可序列化、
/// 可直接作为命令返回值的形态。设备/平台不可用时返回
/// [`NavMoveOutcome::Unavailable`]（本模块不触平台，但仍约定该变体）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NavMoveOutcome {
    /// 已移动高亮到新元素并尝试激活。
    Moved {
        /// 命中元素序号（在扫描快照中的下标）。
        index: usize,
        /// 是否确认激活（Invoke 成功或 SetFocus 后元素持有焦点）。
        activated: bool,
        /// 命中元素的归一化矩形（overlay 画框用）。
        rect: NormalizedRect,
    },
    /// 该方向没有可导航元素。
    NoElement,
    /// 目标进程已退出或无窗口。
    TargetExited,
    /// 无法访问 UIA 树（附原因文本）。
    Failed {
        reason: String,
    },
    /// 设备/平台不可用（非 Windows，或无法解析前台进程）。
    Unavailable,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(x: f64, y: f64, w: f64, h: f64) -> NavElement {
        NavElement {
            rect: NormalizedRect {
                x,
                y,
                width: w,
                height: h,
            },
            etype: NavElementType::Button,
        }
    }

    fn cursor(x: f64, y: f64, w: f64, h: f64) -> NormalizedRect {
        NormalizedRect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn picks_closest_in_right_direction() {
        let cursor = cursor(0.0, 0.0, 10.0, 10.0); // center (5,5)
        let elems = vec![
            element(20.0, 0.0, 10.0, 10.0), // right, center (25,5) → 距离 20
            element(40.0, 0.0, 10.0, 10.0), // right, center (45,5) → 距离 40
            element(0.0, 20.0, 10.0, 10.0), // below
        ];
        assert_eq!(pick_at(&cursor, NavDir::Right, &elems), Some(0));
        assert_eq!(pick_at(&cursor, NavDir::Down, &elems), Some(2));
    }

    #[test]
    fn ignores_elements_not_ahead() {
        let cursor = cursor(5.0, 5.0, 10.0, 10.0); // center (10,10)
        let elems = vec![
            element(0.0, 0.0, 5.0, 5.0), // top-left, center ~(2.5,2.5) → 不在右侧
            element(20.0, 20.0, 10.0, 10.0),
        ];
        // 右边只有第二个。
        assert_eq!(pick_at(&cursor, NavDir::Right, &elems), Some(1));
        // 左边只有第一个。
        assert_eq!(pick_at(&cursor, NavDir::Left, &elems), Some(0));
    }

    #[test]
    fn aligned_wins_tie_in_primary() {
        // 两个元素主方向距离几乎相同，正交更对齐者胜。
        let cursor = cursor(0.0, 0.0, 10.0, 10.0); // center (5,5)
        let elems = vec![
            element(15.0, 0.0, 10.0, 10.0), // right, center (20,5), dy=0 → 对齐
            element(15.0, 20.0, 10.0, 10.0), // right, center (20,25), dy=20 → 不对齐
        ];
        let picked = pick_at(&cursor, NavDir::Right, &elems).unwrap();
        assert_eq!(picked, 0);
    }

    #[test]
    fn prefers_earlier_when_ties() {
        // 完全相同位置 → 返回第一个。
        let cursor = cursor(0.0, 0.0, 10.0, 10.0);
        let elems = vec![
            element(20.0, 0.0, 10.0, 10.0),
            element(20.0, 0.0, 10.0, 10.0),
        ];
        assert_eq!(pick_at(&cursor, NavDir::Right, &elems), Some(0));
    }

    #[test]
    fn empty_or_none_ahead_returns_none() {
        let cursor = cursor(0.0, 0.0, 10.0, 10.0);
        assert_eq!(pick_at(&cursor, NavDir::Right, &[]), None);
        let elems = vec![element(0.0, 40.0, 10.0, 10.0)];
        assert_eq!(pick_at(&cursor, NavDir::Up, &elems), None);
    }

    #[test]
    fn prefer_invoke_only_for_invokable_types() {
        assert!(NavElementType::Button.prefer_invoke());
        assert!(NavElementType::Link.prefer_invoke());
        assert!(!NavElementType::TreeItem.prefer_invoke());
        assert!(!NavElementType::Other.prefer_invoke());
    }
}