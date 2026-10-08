# CPR 账号能力检测插件

支持 CPR `v3.21.2` 和 `v3.21.3`，安装包面向 Linux x86_64，二进制静态链接 musl，无需在服务器安装 Rust 或 Node.js

## 安装与使用

1. 在 CPR 的「插件管理」选择「上传包」，上传 `dist/` 中的 `.tar.gz`
2. 确认来源并安装，默认配置无需填写参数
3. 从插件导航打开「账号能力检测」
4. 选择有权限访问被测账号和模型的 Client Key，再选择测试模型与推理档位
5. 单行点击「测试」，或勾选多个账号后点击「测试勾选账号」

题库、调用参数编辑后点击「保存设置」可供下次打开使用，测试会使用页面上的当前参数

运行测试会消耗所选 Key 的预算，账号权限、分组、额度与并发限制由 CPR 执行

## 题库与判定

内置糖果题保留用户题目原文，预期答案为 `21`，仅忽略首尾空白

| 回答 | 结果 |
| --- | --- |
| `21` 或首尾带空白的 `21` | 通过 |
| 其他整数 | 答案错误 |
| `答案是21`、`21个`、附带解释或空回答 | 格式错误 |
| 超时、限流、授权、额度或响应未正常完成 | 调用错误 |

最多保存 20 道题，支持精确匹配、包含内容、正则匹配和人工查看，题目可单独启用

单次回答未通过只提示疑似异常，不能单独证明账号降智，调用错误不计入答案判定分母

## 批量测试与历史

并发为 1–4，重复为 1–5 次，每题超时为 5–90 秒，单批最多 2000 次调用

取消只停止尚未执行的任务，已经执行的测试会正常收尾，关闭页面会丢失剩余队列，已保存结果仍可从历史查看

历史保留最近 100 次测试，每条保存当时的题目、判定规则、账号、Key ID、模型、推理档位、回答、耗时和错误原因，可筛选并导出 JSON 文本

提示词限制为 8 KiB，预期答案为 2 KiB，题库与设置合计不超过 48 KiB，回答最多保存 8 KiB，超长回答标记为调用错误，避免对截断内容判分

账号通过 SDK 的 `account_id` 强制锁定，CPR 每次尝试都会重新校验该约束，SDK 不提供单独的实际账号回传字段

结果写入发生版本冲突时只合并历史，不重新调用模型，同一测试 ID 的并发提交会被拒绝，已保留的测试 ID 再次提交会返回既有结果

## 构建

需要 Rust `1.97.0`、Node.js `24` 以上和目标版本 CPR 的 `cpr-plugin` CLI，SDK 源码已固定并附带在 `vendor/`

```bash
cd frontend
npm ci --legacy-peer-deps
npm run lint
npm run build
cd ..
cargo fmt --manifest-path backend/Cargo.toml -- --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path backend/Cargo.toml --locked
cargo build --manifest-path backend/Cargo.toml --release --locked
node scripts/protocol-smoke.mjs backend/target/release/cpr-account-check
```

Linux x86_64 上使用 `bash scripts/package.sh` 构建并打包，`PLUGIN_CLI` 可指定 `cpr-plugin` 的路径

开发页面可用 `npm run dev -- --port 5173`，`?demo` 开启模拟数据预览，模拟代码不进入生产构建

插件只声明 `management` 能力，题库和历史使用插件私有状态，无需模型请求绑定，不自动停用账号或修改分组
