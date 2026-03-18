# 项目规范总览

本项目是 Rust 桌面客户端 / 本地优先架构，技术栈固定为：

- UI shell: egui / eframe
- Visualization renderer: Bevy
- Backend/service layer: Rust
- Database: PostgreSQL
- Platform priority: Windows

规范目标：

1. 让 AI 在修改代码前先理解项目的实际架构边界。
2. 避免把 Web/DOM/React 模式错误迁移到 egui + Bevy。
3. 保证 PostgreSQL 的 schema、事务、索引、迁移遵循统一规则。
4. 所有新增代码优先服从本项目现有模块边界，而不是引入新范式。

阅读顺序：

1. frontend/index.md
2. backend/index.md
3. guides/index.md
