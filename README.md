# AutoClicker

轻量、克制的 Windows 自动连点器。Rust 负责计时与点击注入，Vue 3 只负责界面——任何一次点击都不经过 JavaScript 定时器，因此 1 ms 间隔也不会漂移。

## 功能

- **点击控制**：间隔（ms ⇄ Hz 双向换算）、单次 / 双击 / 按住、左/中/右/X1/X2 按键、点击次数上限
- **坐标模式**：跟随光标，或固定物理像素坐标（支持多显示器 + DPI 缩放，界面按逻辑像素显示）
- **节奏变化**：线性变速、随机区间、间隔抖动、坐标抖动
- **多点位序列**：最多 100 步，可循环，每步独立按键 / 次数 / 按住时长 / 间隔
- **键盘连发**：模拟指定按键而非鼠标点击
- **触发与保护**：倒计时、延迟开始、运行时长上限、前台窗口绑定（标题 / 进程名匹配后才点击）
- **全局热键**：默认 `F6` 开始/停止、`F7` 紧急中止，窗口未聚焦或最小化时同样有效
- **其他**：开机自启、深浅色主题、中英双语、配置导入/导出

## 环境要求

- Windows 10 / 11（点击注入基于 Win32 `SendInput`，不跨平台）
- Rust 1.85+（edition 2024）
- Node.js 20+ 与 pnpm
- 若需构建安装包：Visual Studio Build Tools（MSVC）与 WebView2 运行时

## 开发

```bash
pnpm install
pnpm tauri dev
```

## 构建

```bash
pnpm tauri build
```

产物位于 `src-tauri/target/release/bundle/`。`pnpm build` 只做前端类型检查与打包（`vue-tsc --noEmit && vite build`）。

## 目录结构

```
src/                    前端（Vue 3 + Pinia + vue-i18n）
  views/                MainView（连点设置）/ SettingsView（设置、备份、关于）
  stores/               settings（持久化与 Rust 同步）/ runtime（引擎状态镜像）
  components/           设计系统控件（Button/Segmented/Toggle/...）
  locales/              zh-CN / en-US
src-tauri/src/
  lib.rs                插件注册、命令注册、启动时恢复设置
  commands.rs           全部 IPC 命令（含配置导入/导出）
  state.rs              设置模型、运行统计、日志环形缓冲
  hotkey.rs             全局热键注册（失败自动回滚）
  window_bind.rs        前台窗口匹配
  clicker/
    mod.rs              SessionConfig：设置 → 引擎快照、间隔计算
    engine.rs           点击循环线程（tick 事件、倒计时、序列）
    inject.rs           SendInput 封装、DPI、虚拟键映射
    sequence.rs         多点位序列编排
```

## 关键设计

- **设置在启动运行后被冻结**：`start_clicking` 会把设置快照成不可变的 `SessionConfig`，运行中修改界面不会与点击循环竞争。
- **停止是即时的**：`stop_clicking` 置位取消标志并 `join` 引擎线程，返回后不会再有点击落地。
- **坐标一律使用物理像素**，只在展示层按显示器 DPI 换算，避免缩放比例导致的偏移。
- **不点击自身窗口**：注入前会检查目标点所属进程，命中本程序窗口时跳过该次点击。否则"跟随光标"模式下连点会跟着鼠标落在界面按钮上，注入的按下/抬起事件会把物理点击冲掉，用户就再也点不动"停止"了。修复后界面按钮始终可用，`F6`/`F7` 仍然是最快的停止方式。
- **错误是结构化的**：Rust 侧只返回稳定错误码 + 参数（如 `inputBlocked`、`hotkeyConflict`），由前端翻译成当前语言的提示。
- **配置文件**：`settings.json` 存于应用数据目录（设置页"关于 → 数据目录"可打开）。导入/导出使用 Rust 侧的原生对话框，允许部分字段缺失（缺省值来自 `Settings::default()`）。

## 常见问题

- **点击无效 / 报 `inputBlocked`**：目标窗口以管理员权限运行。Windows 的 UIPI 阻止低完整性进程向其注入输入，请以管理员身份启动本程序。
- **热键不生效**：热键被其他程序占用时会提示 `hotkeyConflict`，换一个组合即可；重启后设置会自动重新注册。
- **间隔 1 ms 但实际更慢**：注入本身有开销，程序会在落后时跳过补偿点击而不是补发突发点击，因此实际速率是能达到的上限。

## 许可证

MIT
