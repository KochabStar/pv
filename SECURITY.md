# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |
| < latest | :x:                |

## Reporting a Vulnerability

请通过 GitHub Security Advisory 报告漏洞：

1. 打开 [Security Advisories](https://github.com/loonghao/pv/security/advisories/new)
2. 填写漏洞详情
3. 提交后我们会私下联系确认

请勿公开披露，直到我们确认并修复。

## Scope

- `pv` 二进制本身的漏洞（权限提升、命令注入等）
- 安装脚本的安全问题（`scripts/install.ps1`）
- CI/CD 流程中的密钥泄露风险

## 期望

- 我们会在 **5 个工作日内** 确认收到报告
- 修复时间取决于严重程度，关键漏洞尽快修复并发布补丁版本
- 确认修复后，会致谢报告者（如希望公开致谢）
