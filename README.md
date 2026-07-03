# 🪳 蟑螂提醒 (Cockroach Reminder) — Rust + iced

> [!CAUTION]
> **⚠️ 重要警告：本软件仅限本人使用，严禁私自安装到他人的电脑上！**
> **⚠️ WARNING: FOR PERSONAL USE ONLY. PRIVATE INSTALLATION ON OTHERS' COMPUTERS IS STRICTLY PROHIBITED.**

采用 **Rust + [iced](https://iced.rs)** 构建的定时休息提醒工具：定时召唤满屏蟑螂，强制你去休息。

## 🌟 主要功能

- **定时提醒**：自定义工作时长（1–120 分钟），时间一到，蟑螂就会从屏幕四周爬出。
- **自定义召唤**：最高支持同时召唤 **50 只** 蟑螂（每块显示器各 50 只）。
- **灵活配置**：调整蟑螂的数量、大小、移动速度（5–50%）、快速蟑螂概率，以及休息时长。
- **智能循环**：休息结束后自动重新开始工作计时。
- **暂停/恢复**：随时暂停或恢复计时。
- **菜单栏常驻**：通过系统托盘（菜单栏）🪳 图标控制，无 Dock 图标。
- **多显示器支持**：在每一块显示器上都铺满透明、点击穿透、置顶的覆盖层。
- **多平台支持**：**macOS**（点击穿透 + 屏保级别置顶 + 全工作区）、**Windows**（置顶）、**Linux/X11**（置顶 + Xinerama 多屏）。
- **系统通知**：休息开始时弹出静默通知（可关闭）。
- **开机自启动**：支持配置开机自动启动（macOS）。
- **完整中文界面**：深色主题设置面板。

## 🛠️ 技术栈

| 技术 | 选型 |
|---|---|
| GUI / 渲染 | **iced 0.14**（`canvas` + `wgpu`） |
| 动画 | iced 订阅（16ms 动画时钟） |
| 托盘菜单 | **tray-icon** |
| 多平台窗口集成 | **raw-window-handle** 获取原生句柄，各平台分别处理 |
| macOS 覆盖层 | **objc2** 调用 AppKit（`setIgnoresMouseEvents` / `setLevel` / `setCollectionBehavior` / `NSScreen.screens`） |
| Windows 覆盖层 | **windows-sys** Win32（`EnumDisplayMonitors` / `SetWindowPos HWND_TOPMOST`） |
| Linux/X11 覆盖层 | **x11rb**（Xinerama 多屏 / `_NET_WM_STATE_ABOVE`） |
| 设置持久化 | **serde_json** → `~/.config/com.cockroach.reminder/config.json` |
| 系统通知 | **notify-rust** |
| 图片解码 | **image**（PNG 帧编译期内嵌） |
| 随机 | **rand** |

## 📐 代码结构

```
src/
  main.rs        — iced daemon：状态机、update / view / subscription、设置窗口管理、托盘轮询
  config.rs      — Settings（默认值/范围裁剪/JSON 读写）
  timer.rs       — 计时状态机（Idle / Running / Break / Paused）
  cockroach.rs   — 单只蟑螂物理动画（出生位置/速度/帧动画/越界重生）
  overlay.rs     — 覆盖层 canvas Program（旋转+平移绘制帧）
  settings_ui.rs — 设置窗口 UI 组件（滑块/复选框/按钮布局）
  tray.rs        — 托盘图标与菜单（动态刷新状态标签）
  constants.rs   — 13 帧动画帧 / 图标资源的编译期内嵌
  platform.rs    — 跨平台窗口集成
    ├── macOS    — Dock 隐藏、点击穿透、屏保级别置顶、全空间、多屏枚举
    ├── Windows  — EnumDisplayMonitors 检测多屏、SetWindowPos 置顶
    └── Linux    — Xinerama 多屏检测、_NET_WM_STATE_ABOVE EWMH 置顶
assets/
  frames/        — 13 帧蟑螂走步动画 PNG
  icon.png       — 应用图标
  trayIcon*.png  — 托盘菜单栏图标（含 @2x 模板图）
```

## 🚀 构建与运行

```bash
cargo run --release
```

启动后会出现在菜单栏（不显示 Dock 图标）。右键/点击 🪳 图标：

- **暂停 / 恢复计时**
- **🪳 立即开始休息 (召唤蟑螂)**
- **⚙ 设置…** — 打开设置窗口（休息间隔、显示时长、数量、大小、速度、快速概率、自动启动、开机自启、通知开关）
- **❌ 退出**

> ### 平台说明
>
> | 平台 | 覆盖层特性 | 多屏检测 | 实现 |
> |---|---|---|---|
> | **macOS** | ✅ 透明 + 点击穿透 + 屏保级别置顶 + 全工作区 | `NSScreen.screens` | objc2 AppKit |
> | **Windows** | ✅ 透明 + 置顶（无点击穿透） | `EnumDisplayMonitors` | windows-sys Win32 |
> | **Linux/X11** | ✅ 透明 + 置顶（无点击穿透） | Xinerama（x11rb） | x11rb EWMH |
>
> 非 macOS 平台因无法实现点击穿透，休息期间会拦截鼠标操作——这被设计为**强制休息**行为，而非缺陷。

联系邮箱：1697859639@qq.com
