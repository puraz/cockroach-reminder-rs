# 🪳 蟑螂提醒 — 内存泄漏分析与修复计划

## 一、项目架构全景

```
main.rs (iced daemon)
  ├── App.state: { settings, timer, frames, overlays, tray, ... }
  ├── subscription() → Tick(1s) + PollTray(150ms) + Anim(16ms, 仅活跃时)
  ├── update() → Message dispatch
  ├── view() → widget tree (settings 窗口 / overlay canvas)
  │
  ├── config.rs    → Settings 持久化 (JSON)
  ├── timer.rs     → Phase 状态机 (Idle/Running/Break/Paused)
  ├── cockroach.rs → 单只蟑螂物理动画
  ├── overlay.rs   → OverlayCanvas (canvas Program, draw_image)
  ├── constants.rs → 13 帧 PNG 内嵌字节 + 图标
  ├── tray.rs      → 系统托盘图标/菜单
  ├── settings_ui.rs → 设置窗口 UI
  └── platform.rs  → macOS objc2 / Windows Win32 / Linux x11
```

### 关键生命周期

```
启动
  └─ load_frames_task() → 后台线程解码 13 帧 PNG → frames: Vec<SpriteFrame>
  └─ Tray::new() → 托盘常驻

定时到达 (EnteredBreak)
  └─ spawn_overlays()
       └─ show_overlays(screens)
            ├─ 复用已有的 overlay window (隐藏→显示)  ← 快速响应
            ├─ 或创建新的 overlay window               ← 仅首次较慢
            └─ 每个窗口 seed N 只 Cockroach

动画循环 (16ms)
  └─ Message::Anim → Cockroach::update → 更新位置/帧
  └─ view() → OverlayCanvas::draw()
       └─ for each Cockroach: f.draw_image(rect, Image::new(sprite.handle.clone()))

休息结束 (EnteredRunning)
  └─ close_overlays()
       ├─ ov.active = false
       ├─ ov.cockroaches.clear()
       └─ window::set_mode(Hidden)  →  不关闭窗口，仅隐藏 ← 为快速恢复
```

---

## 二、UX 约束

| 约束 | 要求 | 现状 |
|------|------|------|
| **启动速度** | 应用启动不能太慢 | ✅ 帧加载在后台线程，不阻塞 UI |
| **召唤响应** | 定时到达或点击"立即开始休息"时，蟑螂要**快速出现** | ✅ 当前复用隐藏窗口 → 瞬间显示；❌ 若改为关闭+重建窗口则变慢 |

**核心结论**：任何修复方案不能牺牲上述两点 UX。

---

## 三、内存泄漏根因分析

### 🔴 L1 — 最可能：Overlay 窗口的 wgpu Surface/SwapChain 在隐藏后未释放

**位置**: [`src/main.rs:432`](src/main.rs:432)

```rust
fn close_overlays(&mut self) -> Task<Message> {
    let tasks = self.overlays.iter_mut().map(|ov| {
        ov.active = false;
        ov.cockroaches.clear();
        window::set_mode::<Message>(ov.id, window::Mode::Hidden)
    });
    Task::batch(tasks)
}
```

**问题链**:

1. 每个 Overlay 窗口对应一个 wgpu `Surface` + `SwapChain`（每个 N 屏 ≈ 1920×1080×4bytes ×2~3 buffers ≈ **24-36MB/窗口**）
2. `set_mode(Hidden)` 隐藏窗口但 **不销毁 Surface**
3. 下次 break 时 `set_mode(Windowed)` 恢复，可能创建 **新的 SwapChain** 而旧的不释放
4. macOS 的 NSWindow 即使隐藏也持有 CALayer 和 Metal 上下文，GPU 背面内存可能不会被立即回收

**⚠️ UX 冲突**: 若采用"关闭窗口替代隐藏"的修复方案，则每次召唤蟑螂都需要：
- 创建 N 个新 NSWindow
- 调用 `platform::configure_overlay()`（objc2 消息发送：`setIgnoresMouseEvents`、`setLevel`、`setCollectionBehavior`、`setFrame` 等）
- iced 内部创建新的 wgpu Surface + SwapChain

→ 导致召唤蟑螂时有**可感知的延迟**（预估 50-200ms）

### 🟧 L2 — 中等可能：iced 0.14 canvas 的 GPU 纹理缓存失效

**位置**: [`src/overlay.rs:69`](src/overlay.rs:69)

```rust
f.draw_image(rect, Image::new(sprite.handle.clone()));
```

**分析**（已通过源码验证 ICED 内部实现）：
- `iced_core::image::Handle::Rgba` 的 `pixels` 字段类型是 `Bytes`，它是 `bytes::Bytes`（引用计数，`Arc<[u8]>` 的封装）。**clone() 只增加引用计数，不拷贝数据。**
- Handle 的 `Id` 在 clone 后保持不变。iced 的 GPU 纹理缓存按 `Id` 索引，**理论上应该命中缓存**。
- 但 canvas 的 `draw_image` 路径可能不走 widget 层的 texture cache，而是独立管理纹理。如果独立缓存也有 Bug，则每帧都会创建新 GPU 纹理。

**✅ `Handle::clone()` 本身不是泄漏源。但如果 canvas 纹理缓存有 bug，则 draw_image 会在 GPU 侧持续分配。**

### 🟧 L3 — 中等可能：`overlays: Vec<Overlay>` 无限增长

**位置**: [`src/main.rs:355`](src/main.rs:355)

```rust
// 对新显示器创建新 overlay
self.overlays.push(Overlay { id, width: w, height: h, ... });

// 多余的 overlay 早期仅隐藏不移除
for ov in self.overlays.iter_mut().skip(screen_count) {
    ov.active = false;
    ov.cockroaches.clear();
    tasks.push(window::set_mode::<Message>(ov.id, window::Mode::Hidden));
}
```

**问题**: 如果显示器配置动态变化（插拔外接屏、合盖/开盖），每次 break 可能创建新窗口，旧窗口永远留在 `overlays` 中。

**✅ 已修复**: 见下方「已实施变更」章节。

### 🟨 L4 — 低可能：iced canvas Geometry 积累

每次 `draw()` 返回一个新的 `Vec<Geometry>`。如果 iced 内部的 renderer 在 compositing 过程中未及时释放旧 geometry 的 GPU command buffer 引用，可能积累。

### 🟨 L5 — 低可能：字体/Text 缓存增长

iced 的 text renderer 使用 `cosmic-text` / `wgpu_glyph` 做字体 rasterization。PingFang SC 是 CJK 字体，glyph 较多，但通常不会无限制增长。

---

## 四、排查方法

### 4.1 二分法隔离验证（推荐优先执行）

**测试 A** — 禁用 canvas 绘制（定位是否 GPU 纹理上传导致）
```rust
// 在 overlay.rs draw() 中注释掉所有 draw_image 调用
fn draw(...) -> Vec<Geometry> {
    let frame = Frame::new(renderer, bounds.size());
    // 不画任何蟑螂，只返回空白 frame
    vec![frame.into_geometry()]
}
```
→ 如果泄漏消失 → 问题在 `draw_image` / GPU texture 上传（L2）
→ 如果泄漏仍在 → 问题在窗口本身（L1）

**测试 B** — 在测试 A 基础上，close_overlays 改为关闭窗口
```rust
fn close_overlays(&mut self) -> Task<Message> {
    let ids: Vec<_> = self.overlays.drain(..).map(|ov| ov.id).collect();
    Task::batch(ids.into_iter().map(window::close::<Message>))
}
```
→ 如果泄漏消失 → 问题在 wgpu Surface/SwapChain 残留（L1）
→ 如果泄漏仍在 → 问题在 iced 框架内部

### 4.2 macOS 内存分析工具

```bash
# 实时查看 RSS（物理内存）变化
while sleep 1; do ps -p $(pgrep -f cockroach-reminder) -o rss= | awk '{printf "\rRSS: %.1f MB", $1/1024}'; done

# 查看完整 VM region 分布（IOKit/Metal 区域）
vmmap $(pgrep -f cockroach-reminder) --summary

# 查看 Metal/GPU 内存
# Xcode → Debug → GPU Memory Report
```

### 4.3 Instruments 工具

使用 macOS 自带的 **Instruments**:
- **Allocations**: 跟踪堆分配增长（观察多次 break cycle 后的 persistent allocations）
- **Leaks**: 检查无法到达的对象
- **GPU Memory**: Metal 的显存使用
- **VM Tracker**: 查看 VM region 类型分布（重点关注 `IOKit` 和 `Metal` 类型）

---

## 五、已实施变更

### 变更 1（已完成）：多余 overlay 关闭+移除

**位置**: [`src/main.rs:423-427`](src/main.rs:423)

**之前**：多余 overlay 仅 `set_mode(Hidden)`，永远留在 `overlays` Vec 中。

```rust
for ov in self.overlays.iter_mut().skip(screen_count) {
    ov.active = false;
    ov.cockroaches.clear();
    tasks.push(window::set_mode::<Message>(ov.id, window::Mode::Hidden));
}
```

**之后**：多余 overlay 调用 `window::close()` 并从 Vec 中 `drain` 移除。

```rust
let excess_ids: Vec<_> = self.overlays.drain(screen_count..).map(|ov| ov.id).collect();
for id in excess_ids {
    tasks.push(window::close::<Message>(id));
}
```

**UX 影响**: ✅ **无影响**（仅在显示器数量减少时触发，概率低）

### 变更 2（已回退）：SpriteFrame.handle 包 Arc

**结论**: `image::Handle::clone()` 已是 O(1) 操作（`Bytes` 引用计数，`Id` 保持），Arc 包装不必要且无法通过编译（`Image::new` 不支持 `Arc<Handle>`）。

---

## 六、下一步排查执行步骤

```
Step 1: 编译并运行应用，观察修复 3 后的内存表现
  └─ 用 ps/vmmap 观察多次 break cycle 后的 RSS

Step 2: 执行二分法测试 A
  └─ 注释 overlay.rs draw() 中的 draw_image 调用
  └─ 确定泄漏在 GPU 纹理上传还是窗口本身

Step 3: 根据测试结果
  └─ 如泄漏在纹理上传 → 检查 iced canvas texture cache 实现
  └─ 如泄漏在窗口 → 考虑升级 iced 版本或提 issue
```

---

## 七、预防措施

1. **每帧分配审计**：`draw()` 函数中避免任何大块内存分配
2. **窗口生命周期管理**：理解隐藏窗口的 GPU 资源释放策略
3. **监控**：在开发过程中加入内存 profiling 步骤
4. **升级 iced**：iced 0.14 处于早期阶段，后续版本可能修复了 texture cache 和 window lifecycle 的问题

---

## 八、相关代码位置速查

| 文件 | 行号 | 说明 |
|------|------|------|
| [`src/main.rs`](src/main.rs:432) | 432 | `close_overlays()`: 仅隐藏窗口未关闭 |
| [`src/main.rs`](src/main.rs:355) | 355-430 | `show_overlays()`: overlay 生命周期管理 |
| [`src/main.rs`](src/main.rs:423) | 423-427 | ✅ 已修复：多余 overlay 改为 close+remove |
| [`src/main.rs`](src/main.rs:526) | 526-536 | `load_frames_task()`: 后台线程加载帧 |
| [`src/main.rs`](src/main.rs:538) | 538-588 | `load_sprite_frames()`: 解码→裁剪→resize→from_rgba |
| [`src/overlay.rs`](src/overlay.rs:39) | 39-73 | `OverlayCanvas::draw()`: 整个绘制循环 |
| [`src/overlay.rs`](src/overlay.rs:69) | 69 | `draw_image`: 核心嫌疑点（纹理缓存） |
