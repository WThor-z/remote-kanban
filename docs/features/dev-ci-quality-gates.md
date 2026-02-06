# 开发能力：CI 门禁与质量校验

## 概述
为防止回归，仓库在推送与合并请求（Pull Request）时自动执行安装、构建、测试与文档一致性校验，确保关键链路在合并前可验证。

## 入口
- UI：无
- API：GitHub Actions 工作流 `\.github/workflows/ci.yml`
- CLI：无

## 行为与边界
- 在 `main`、`dev` 分支推送，以及所有合并请求上触发。
- 执行 `pnpm install --frozen-lockfile`、`pnpm run check:docs:features`、`pnpm run test:scripts`、`pnpm -r run build`、`pnpm -r run test`、`cargo test --manifest-path crates/Cargo.toml`。
- 仅负责校验与阻断，不负责自动修复失败项。

## 数据与存储影响
- 无业务数据写入；仅生成 CI 运行日志与临时构建产物。

## 权限与风险
- 需要仓库 CI 运行权限；流程失败会阻断合并。

## 可观测性
- 通过 GitHub Actions 运行记录展示每个步骤的日志与状态。

## 测试与验证
- 本地可依次执行 `pnpm run check:docs:features`、`pnpm run test:scripts`、`pnpm -r run build`、`pnpm -r run test`、`cargo test --manifest-path crates/Cargo.toml` 进行等价验证。

## 相关变更
- `\.github/workflows/ci.yml`
- `package.json`（`test:scripts` 脚本）
