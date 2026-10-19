# 交付校验

## 0.1.4（已构建，待安装验证）

校验日期：2026-10-19。本版修复 CPR `v3.29.3` 上插件「无法启动」，界面提示「插件清单或协议版本不兼容。请使用当前 SDK 重新构建」。

- 根因：CPR `v3.29.0` 起 `plugin-host-compatibility.json` 的 `protocol_versions` 由 2 改为 4，`management` 能力合同由 v1 改为 v2。0.1.3 的二进制握手仍发送协议 2，清单也声明协议 2 与 management v1；宿主 `runtime/src/rpc/session.rs` 要求 `handshake.protocol_version == 4`、`package.protocol_version == handshake.protocol_version`、清单与握手的 `contributes` 完全一致，任一不符即拒绝启动。
- 依据：SDK 固定版本更新为 CPR `v3.29.3`（提交 `16d8a3f87309d9346a5f204673390fe20478b40e`）。diff 中 `manifest.rs` 未改；`message.rs` 的 `PROTOCOL_VERSION` 为 4，`Call.params`、`Result.result`、`Callback.params`、`Handshake.configuration` 改为帧载荷前缀（`data_bytes`），`context` 新增 `RequestContext`；`capability.rs` 中 management 合同为 `[2]`；插件用到的 `call/host.rs`、`call/data.rs`、`call/management.rs` 与 `client/` 调用签名未变，后端无需改业务代码。
- 已改：`vendor/gateway-plugin-sdk` 源码替换为 v3.29.3（新增 `base64` 依赖）；`plugin.json` 版本 0.1.4、`engines` 为 `>=3.29.3, <4.0.0`、`contributes.management.version` 为 2；`backend/Cargo.toml`、`frontend/package.json`、`frontend/package-lock.json`、`README.md` 同步版本与宿主范围；`scripts/protocol-smoke.mjs` 改为协议 v4 帧格式。
- 已验证：`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`（4 组）通过；前端 ESLint 与 `vue-tsc --noEmit` 通过；协议冒烟测试对 release 构建通过，覆盖协议 v4 握手、management v2 注册、账号分页、设置冲突、账号锁定、判分、调用错误、历史合并、重复提交、超时关闭、取消恢复与历史上限。
- 已构建：`cargo build --release --locked --target x86_64-unknown-linux-musl` 通过，`bin/plugin` 为 ELF64 x86-64、`ET_DYN` 静态可执行（无 `PT_INTERP`）；用 v3.29.3 的 `cpr-plugin` 打包。
- 已校验：归档内 `plugin.json` 版本 0.1.4、`engines` 为 `>=3.29.3, <4.0.0`、`contributes.management.version` 为 2、`package.protocolVersion` 为 4、`package.target` 为 `linux/x86_64`；四个文件摘要与承载内容逐一匹配；整包 SHA-256 `c2e82412b7df0e8639702cf3ac6ad984143576603bbd9b9edd525654b9652d72` 与 `.sha256` 一致。
- 已复用：前端源码未改，`web/app.css`、`web/app.js`、`web/index.html` 的摘要与 0.1.3 包内资源完全相同。
- 未验证：未在真实 CPR 3.29.3 实例上安装本版本，未执行真实模型调用；安装后需确认插件状态由「异常」变为可用，并从插件页面执行一次单账号测试。

## 0.1.3（已构建并发布）

校验日期：2026-10-13。本版只放宽宿主版本声明，`plugin.json` 由 `>=3.21.2, <3.22.0` 改为 `>=3.21.2, <4.0.0`，版本号同步到 0.1.3（`plugin.json`、`backend/Cargo.toml`、`backend/Cargo.lock`、`frontend/package.json`、`frontend/package-lock.json`、`README.md`）。

- 依据：逐一比对 CPR `v3.21.3`、`v3.22.0`、`v3.23.0` 的 `plugin-host-compatibility.json`，`manifest_schema_versions`、`protocol_versions` 与 `management` 能力版本三版一致；两版 `plugin-api-deprecations.json` 中与本插件相关的条目为空。
- 依据：SDK 源码 diff 仅涉及 `capability.rs`（middleware v3 与 upstream_adapter v1 收口）、`call/services/settings.rs`、`call/observation.rs`、`call/model/facts.rs`、`upstream_adapter/codec.rs`；插件使用的 `call/host.rs`、`call/data.rs`、`call/model/`、`client/` 与管理页合同均未改动。
- 依据：宿主侧 `package_for()`（唯一会因版本范围不符返回 `ManifestError::Platform` 的入口）在整条安装与运行链路上没有被调用，`plugin_system_update` 预检只把版本范围不符列为待确认项。升级到 3.22.x / 3.23.0 时的提示来自插件自身声明，不是协议不兼容。
- 已构建：`cargo build --release --locked --offline --target x86_64-unknown-linux-musl` 通过，`bin/plugin` 为 ELF64 x86-64、`ET_DYN` 静态可执行（无 `PT_INTERP`、无动态 libc 依赖）。
- 已校验：归档内 `plugin.json` 版本 0.1.3、`engines` 为 `>=3.21.2, <4.0.0`、`package.target` 为 `linux/x86_64`、`protocolVersion` 为 2；四个文件摘要与承载内容逐一匹配；整包 SHA-256 `94a3ac2956a79aed03ad75cf64f372815b6e76b7024b5c0af533e3dae4a19692` 与 `.sha256` 及 GitHub 上传后回报的摘要一致。
- 已复用：前端资源未重新构建，`web/app.css`、`web/app.js`、`web/index.html` 的摘要与 0.1.2 完全相同。
- 已发布：`v0.1.3` 标签与 GitHub Release（含 `.tar.gz` 与 `.sha256`）。
- 未验证：未在 CPR 3.22.0 / 3.23.0 实例上安装本版本。

### 本机构建方式（Windows 交叉编译）

本机没有全局 Rust 工具链，构建依赖 `C:\Users\Thinkpad\Documents\Codex\2026-10-08\ni\work\` 下的便携环境（非仓库内容，换机需重建）：

- `. work\rust-env.ps1` 载入 `CARGO_HOME`、`RUSTUP_HOME`、`CARGO_TARGET_DIR`（便携 rustup 1.97.0，含 `x86_64-unknown-linux-musl` target）。
- musl 链接需要显式链接器：`RUSTFLAGS='-C linker=zigcc.cmd -C linker-flavor=ld.lld'`，其中 `work\toolchain\cargo\bin\zigcc.cmd` 只是把参数转交给工具链自带的 `rust-lld.exe`。Rust 自带的 musl 目标已包含匹配的 crt 对象、`libc.a` 与 `compiler_builtins`，因此无需外部 musl 交叉工具链；Zig 的 `cc` 驱动会注入自带 crt 并与之冲突，故不使用。
- 打包：`work\target-cli\release\cpr-plugin.exe package --target x86_64-unknown-linux-gnu`。CLI 仅接受 `-gnu` 这个 triple，`package.target` 由它映射为 `linux/x86_64`；归档内的实际二进制仍是静态 musl 产物（与 0.1.2 的 ELF 结构、大小一致）。
- 0.1.4 本次构建：文件沙箱只允许写工作区，因此用 `.build/env.ps1` 把 `CARGO_HOME`、`CARGO_TARGET_DIR` 指到工作区 `.build/`，`RUSTUP_HOME` 与 `zigcc.cmd` 仍取自便携环境，crates 经 `work\cargo-proxy.mjs`（127.0.0.1:18788）转发。`cpr-plugin` 由 v3.29.3 源码的 `backend/apps/plugin-cli` 复制到 `.build/plugin-cli`，用同一份 `vendor/` SDK 构建；协议冒烟测试通过 `work\process-bridge.py` 的本地 TCP 桥运行（沙箱禁止 Node 直接以管道启动子进程）。
- 发布：`work\tools\github-cli\bin\gh.exe`（`GH_CONFIG_DIR` 指向 `work\tools\github-config`）创建 Release 并上传 assets。

## 0.1.2（已构建并部署验收）

校验日期：2026-10-08；核对范围 CPR v3.21.2 与 v3.21.3。

### 已完成

- 核对 CPR v3.21.2 与 v3.21.3：使用的插件 SDK 一致；SDK 固定为 v3.21.2，并随源码交付。
- Rust 格式检查、严格 Clippy（`-D warnings`）及 4 组单元测试通过。
- 调度器 4 组回归测试通过，验证单题选择、同账号重复串行、不同账号并发、取消及不自动重试。
- Vue/TypeScript 类型检查、ESLint 与生产构建通过；生产资源不包含开发用模拟账号及桥接代码。
- 使用实际 Windows 插件进程和模拟宿主运行协议测试，覆盖握手、管理注册、账号分页、设置冲突、旧设置兼容、选题保存和 capacity 原因保留、指定账号参数、判分、调用错误、并发保存、重复提交、超时关闭及取消恢复。
- 协议测试写入 105 条长回答，确认保留最近 100 条，固定槽位数量有界，状态值和协议元数据不超过声明上限。
- 使用模拟数据确认调用参数只在账号测试页显示，切换到题库和历史页隐藏，返回账号页保留参数；检查新增第二题、单题选择、批量重复测试、选题停用回退、最近检测按题目筛选、取消排队任务、题库编辑、详情、历史筛选及 JSON 复制；检查深色模式与 390 像素窄屏。窄屏表格独立横向滚动，整页不溢出。
- Linux x86_64 静态 musl 发布构建成功。安装包由 CPR 的 `cpr-plugin` CLI 生成，包内文件、前端资源和整体 SHA-256 校验通过，ELF 为 x86_64 且无动态解释器。
- 在用户 Linux x86_64 / CPR v3.21.3 实例完成登录态资源检查和实际页面验收，账号列表、题库入口与模型选项正常加载；普通与条件资源请求的安全响应头均通过宿主校验。

### 尚未验证

v0.1.2 的调度、判分与流式调用验证使用模拟宿主，未部署此版本或执行真实模型测试。用户在 v0.1.0 上的实际测试记录确认第二题返回 capacity 调用错误；本次读取了记录，没有再次调用上游。源码包中的界面截图为模拟数据。

部署后从 CPR「插件管理」上传安装包，选择允许访问对应账号与模型的 Client Key，先执行单账号糖果题，再按需批量测试。真实请求会使用该 Key 的预算。

## 校验命令

以下命令用于 0.1.2；0.1.3 未改动后端与前端代码，沿用同一套流程。执行前先在 Windows 上按上文「本机构建方式」载入便携工具链。

```text
cargo fmt --manifest-path backend/Cargo.toml -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets --locked --offline -- -D warnings
cargo test --manifest-path backend/Cargo.toml --locked --offline
cargo build --manifest-path backend/Cargo.toml --release --locked --offline --target x86_64-unknown-linux-musl
node node_modules/vue-tsc/bin/vue-tsc.js --noEmit
node node_modules/eslint/bin/eslint.js src
node node_modules/vite/bin/vite.js build --configLoader native
node scripts/runner-tests.mjs
node scripts/protocol-smoke.mjs <本地插件可执行文件>
```

交付的 Linux 二进制在 Windows 上用 Rust 自带的 LLD 交叉编译；源码的 `scripts/package.sh` 供 Linux 常规构建使用。协议测试在本环境通过本地进程转接运行，使用的协议数据与脚本直接启动可执行文件时一致。
