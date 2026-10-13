# 交付校验

## 0.1.3（仅源码改动，未构建、未部署）

本版只放宽宿主版本声明，`plugin.json` 由 `>=3.21.2, <3.22.0` 改为 `>=3.21.2, <4.0.0`，版本号同步到 0.1.3（`plugin.json`、`backend/Cargo.toml`、`backend/Cargo.lock`、`frontend/package.json`、`frontend/package-lock.json`、`README.md`）。

- 依据：逐一比对 CPR `v3.21.3`、`v3.22.0`、`v3.23.0` 的 `plugin-host-compatibility.json`，`manifest_schema_versions`、`protocol_versions` 与 `management` 能力版本三版一致；两版 `plugin-api-deprecations.json` 中与本插件相关的条目为空。
- 依据：SDK 源码 diff 仅涉及 `capability.rs`（middleware v3 与 upstream_adapter v1 收口）、`call/services/settings.rs`、`call/observation.rs`、`call/model/facts.rs`、`upstream_adapter/codec.rs`；插件使用的 `call/host.rs`、`call/data.rs`、`call/model/`、`client/` 与管理页合同均未改动。
- 依据：宿主侧 `package_for()`（唯一会因版本范围不符返回 `ManifestError::Platform` 的入口）在整条安装与运行链路上没有被调用，`plugin_system_update` 预检只把版本范围不符列为待确认项。升级到 3.22.x / 3.23.0 时的提示来自插件自身声明，不是协议不兼容。
- 未验证：本环境无 Rust 工具链，未重新构建二进制、未重新打包、未在 3.22.0 与 3.23.0 实例上安装本版本。

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

以下命令用于 0.1.2，0.1.3 沿用同一套流程；`plugin.json` 的宿主范围改动不影响构建步骤。

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
