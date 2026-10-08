# 交付校验

校验日期：2026-10-08。插件版本：0.1.1。

## 已完成

- 核对 CPR v3.21.2 与 v3.21.3：使用的插件 SDK 一致；SDK 固定为 v3.21.2，并随源码交付。
- Rust 格式检查、严格 Clippy（`-D warnings`）及 4 组单元测试通过。
- 调度器 4 组回归测试通过，验证单题选择、同账号重复串行、不同账号并发、取消及不自动重试。
- Vue/TypeScript 类型检查、ESLint 与生产构建通过；生产资源不包含开发用模拟账号及桥接代码。
- 使用实际 Windows 插件进程和模拟宿主运行协议测试，覆盖握手、管理注册、账号分页、设置冲突、旧设置兼容、选题保存和 capacity 原因保留、指定账号参数、判分、调用错误、并发保存、重复提交、超时关闭及取消恢复。
- 协议测试写入 105 条长回答，确认保留最近 100 条，固定槽位数量有界，状态值和协议元数据不超过声明上限。
- 使用模拟数据检查新增第二题、单题选择、批量重复测试、选题停用回退、最近检测按题目筛选、取消排队任务、题库编辑、详情、历史筛选及 JSON 复制；检查深色模式与 390 像素窄屏。窄屏表格独立横向滚动，整页不溢出。
- Linux x86_64 静态 musl 发布构建成功。安装包由 CPR 的 `cpr-plugin` CLI 生成，包内文件、前端资源和整体 SHA-256 校验通过，ELF 为 x86_64 且无动态解释器。
- 在用户 Linux x86_64 / CPR v3.21.3 实例完成登录态资源检查和实际页面验收，账号列表、题库入口与模型选项正常加载；普通与条件资源请求的安全响应头均通过宿主校验。

## 尚未验证

v0.1.1 的调度、判分与流式调用验证使用模拟宿主，未部署此版本或执行真实模型测试。用户在 v0.1.0 上的实际测试记录确认第二题返回 capacity 调用错误；本次读取了记录，没有再次调用上游。源码包中的界面截图为模拟数据。

部署后从 CPR「插件管理」上传安装包，选择允许访问对应账号与模型的 Client Key，先执行单账号糖果题，再按需批量测试。真实请求会使用该 Key 的预算。

## 校验命令

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
