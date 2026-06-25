# 🪳 蟑螂提醒 (Cockroach Reminder) — Rust + iced

> [!CAUTION]
> **⚠️ 重要警告：本软件仅限本人使用，严禁私自安装到他人的电脑上！**
> **⚠️ WARNING: FOR PERSONAL USE ONLY. PRIVATE INSTALLATION ON OTHERS' COMPUTERS IS STRICTLY PROHIBITED.**

这是原 Electron 版 [`cockroach-reminder`](./cockroach-reminder) 的 **Rust + [iced](https://iced.rs)** 完整重写，行为与原版保持一致。

## 🌟 主要功能（与原版一致）

- **定时提醒**：自定义工作时长，时间一到，蟑螂就会从屏幕四周爬出。
- **自定义召唤**：最高支持同时召唤 **50 只** 蟑螂（每块显示器各 50 只）。
- **灵活配置**：调整蟑螂的数量、大小、移动速度、快速蟑螂概率，以及休息时长。
- **智能循环**：休息结束后自动重新开始工作计时。
- **菜单栏常驻**：通过系统托盘（菜单栏）🪳 图标控制，无 Dock 图标。
- **多显示器支持**：在每一块显示器上都铺满透明、点击穿透、置顶的覆盖层。
- **系统通知**：休息开始时弹出静默通知。
- **完整中文界面**。

## 🛠️ 技术栈

| 关注点 | 原版 (Electron) | 本版本 (Rust) |
|---|---|---|
| GUI / 渲染 | HTML/CSS/Canvas + Chromium | **iced 0.13**（`canvas` + `wgpu`） |
| 动画 | `requestAnimationFrame` | iced 订阅（16ms 动画时钟） |
| 托盘菜单 | Electron `Tray`/`Menu` | **tray-icon** |
| 透明/点击穿透/置顶覆盖层 | `BrowserWindow` 选项 | iced 多窗口 + **objc2** 调用 AppKit（`setIgnoresMouseEvents` / `setLevel` / `collectionBehavior`） |
| 多显示器 | `screen.getAllDisplays()` | `NSScreen.screens`（objc2） |
| 设置持久化 | `electron-store` | **serde_json**（`~/Library/Application Support/com.cockroach.reminder/config.json`） |
| 系统通知 | Electron `Notification` | **notify-rust** |

## 📐 代码结构

```
src/
  main.rs        — iced daemon：状态机、update / view / subscription、设置界面、托盘轮询
  config.rs      — Settings（默认值/范围裁剪/JSON 读写），对应 constants.js + store.js
  timer.rs       — 计时状态机（Idle/Running/Break/Paused），对应 timer.js
  cockroach.rs   — 单只蟑螂物理动画，1:1 移植自 overlay.js
  overlay.rs     — 覆盖层 canvas Program（旋转+平移绘制帧），对应 overlay.css/js
  tray.rs        — 托盘图标与菜单，对应 tray.js
  constants.rs   — 帧/图标资源（编译期内嵌）与常量，对应 constants.js
  platform.rs    — macOS 原生窗口/系统集成（Dock 隐藏、点击穿透、置顶、全空间、多屏）
assets/          — 13 帧蟑螂动画 + 图标（取自原项目）
```

## 🚀 构建与运行

```bash
cargo run --release
```

启动后会出现在菜单栏（不显示 Dock 图标）。右键/点击 🪳 图标：

- **暂停 / 恢复计时**
- **🪳 立即开始休息 (召唤蟑螂)**
- **⚙ 设置…** — 打开设置窗口（休息间隔、显示时长、数量、大小、速度、快速概率、各类开关）
- **❌ 退出**

> 平台说明：覆盖层的透明 / 点击穿透 / 置顶 / 全工作区行为通过 AppKit 在 **macOS** 上实现。
> 其它平台可正常编译运行（计时、设置、托盘、动画窗口均可用），但覆盖层的点击穿透等原生特性为降级处理。

## 📜 版权说明

Copyright © 2024 **XUANLI**. All Rights Reserved.
