# AutoClicker

轻量、克制的 Windows 自动连点器。Rust 负责计时与点击注入，Vue 3 只负责界面——任何一次点击都不经过 JavaScript 定时器，因此 1 ms 间隔也不会漂移。

## 功能

- **点击控制**：间隔（ms ⇄ Hz 双向换算）、单次 / 双击 / 按住、左/中/右/X1/X2 按键、点击次数上限
- **坐标模式**：跟随光标，或固定物理像素坐标（支持多显示器 + DPI 缩放，界面按逻辑像素显示）
- **节奏变化**：线性变速、随机区间、间隔抖动、坐标抖动
- **多点位序列**：最多 100 步，可循环，每步独立按键 / 次数 / 按住时长 / 间隔
- **键盘连发**：模拟指定按键而非鼠标点击
- **鼠标跟踪**：光标旁浮层实时显示物理坐标与屏幕缩放，三档——一直跟踪 / 开始后跟踪 / 关闭
- **触发与保护**：倒计时、延迟开始、运行时长上限、前台窗口绑定（标题 / 进程名匹配后才点击）
- **全局热键**：默认 `F6` 开始/停止、`F7` 紧急中止，窗口未聚焦或最小化时同样有效
- **更新**：默认读取 GitHub Release 最新版本，启动后静默检查一次，也可在设置页手动检查并跳转下载
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

## 发布

`.github/workflows/release.yml` 会在推送 `v*` 标签时用 `windows-latest` 构建，并把 `.msi` 与 NSIS `-setup.exe` 上传到同名 GitHub Release：

```bash
# 版本号同步改 src-tauri/tauri.conf.json 的 version 后
git tag v0.1.0
git push origin v0.1.0
```

更新检查默认指向 `src/utils/update.ts` 中的 `GITHUB_REPO`（当前为 `hn1318/AutoClicker`），换仓库时只改这一处即可。它是默认更新源，也是"前往下载"按钮的兜底链接。

安装包未做代码签名，首次运行会出现 SmartScreen 提示。若要做静默自动更新（下载 + 安装而不是跳转浏览器），需要接入 `tauri-plugin-updater` 并配置签名密钥与 `latest.json`，目前用的是轻量的"检查 + 提示 + 打开发布页"方案。

## 目录结构

```
src/                    前端（Vue 3 + Pinia + vue-i18n）
  views/                MainView（连点设置）/ SettingsView（设置、备份、更新、关于）
  stores/               settings（持久化与 Rust 同步）/ runtime（引擎状态镜像）
  components/           设计系统控件（Button/Segmented/Toggle/...）+ CursorOverlay（浮层页）+ ContextMenu
  composables/          useContextMenu / useToast / useErrors / useTheme / useI18n
  utils/                format（版本比较等）/ update（GitHub Release 检查）
  locales/              zh-CN / en-US
src-tauri/src/
  lib.rs                插件注册、命令注册、启动时恢复设置与创建跟踪浮层
  commands.rs           全部 IPC 命令（配置导入/导出、打开链接、跟踪开关）
  state.rs              设置模型、运行统计、日志环形缓冲
  cursor.rs             光标采样线程 + 置顶穿透浮层窗口
  hotkey.rs             全局热键注册（失败自动回滚）
  window_bind.rs        前台窗口匹配
  clicker/
    mod.rs              SessionConfig：设置 → 引擎快照、间隔计算
    engine.rs           点击循环线程（tick 事件、倒计时、序列）
    inject.rs           SendInput 封装、DPI、虚拟键映射
    sequence.rs         多点位序列编排
.github/workflows/      打 tag 自动构建并发布安装包
```

## 关键设计

- **设置在启动运行后被冻结**：`start_clicking` 会把设置快照成不可变的 `SessionConfig`，运行中修改界面不会与点击循环竞争。
- **停止是即时的**：`stop_clicking` 置位取消标志并 `join` 引擎线程，返回后不会再有点击落地。
- **坐标一律使用物理像素**，只在展示层按显示器 DPI 换算，避免缩放比例导致的偏移。
- **不点击自身窗口**：注入前会检查目标点所属进程，命中本程序窗口时跳过该次点击。否则"跟随光标"模式下连点会跟着鼠标落在界面按钮上，注入的按下/抬起事件会把物理点击冲掉，用户就再也点不动"停止"了。修复后界面按钮始终可用，`F6`/`F7` 仍然是最快的停止方式。
- **鼠标跟踪是一个独立窗口**：`cursor-overlay` 是透明、置顶、点击穿透的 webview（`WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`），由 Rust 线程以 50 Hz 采样光标并在指针真正移动时才移动窗口和发事件，指针静止时没有额外开销。窗口在 `setup` 里预先创建（Windows 必须在 GUI 线程建窗），开关只切换显示与采样线程。
- **错误是结构化的**：Rust 侧只返回稳定错误码 + 参数（如 `inputBlocked`、`hotkeyConflict`），由前端翻译成当前语言的提示。
- **右键菜单是自绘的**：WebView2 自带菜单（刷新 / 另存为 / 打印 / 检查）被全局 `preventDefault` 屏蔽；文本框上改为自定义菜单（复制 / 粘贴 / 全选 / 清空），剪贴板走 `tauri-plugin-clipboard-manager`。数值输入框不提供"清空"——它在失焦时会回退到上一个合法值。
- **配置文件**：包名（identifier）为 `top.hn3.autoclicker`，`settings.json` 存于 `%APPDATA%\top.hn3.autoclicker`（设置页"关于 → 数据目录"可打开）。从旧包名 `com.jlhan.autoclicker` 升级时，把旧目录里的 `settings.json` 复制过来即可。导入/导出使用 Rust 侧的原生对话框，允许部分字段缺失（缺省值来自 `Settings::default()`）。

## 常见问题

- **点击无效 / 报 `inputBlocked`**：目标窗口以管理员权限运行。Windows 的 UIPI 阻止低完整性进程向其注入输入，请以管理员身份启动本程序。
- **热键不生效**：热键被其他程序占用时会提示 `hotkeyConflict`，换一个组合即可；重启后设置会自动重新注册。
- **间隔 1 ms 但实际更慢**：注入本身有开销，程序会在落后时跳过补偿点击而不是补发突发点击，因此实际速率是能达到的上限。
- **检查更新失败**：仓库尚未发布 Release 时 GitHub 返回 404；未认证调用 `api.github.com` 也受每小时 60 次限制。此时可在设置页填入自定义 JSON 源（`{ "version": "1.2.3" }`）。
- **鼠标跟踪浮层看不到**：检查"鼠标跟踪"是否被设为"关闭"，或"开始后跟踪"时当前并未运行。

## 许可证

MIT
