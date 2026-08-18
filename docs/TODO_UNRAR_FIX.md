# RAR/CBR 格式支持修复需求

## 问题描述

在实现 Phase 3 过程中，发现 `unrar` crate 的 API 与我们的代码不兼容，导致编译错误。当前已临时禁用 RAR 支持以确保项目编译通过。

## 错误详情

### 编译错误信息
```rust
error[E0599]: no method named `open_for_listing` found for struct `unrar::Archive` in the current scope
  --> src/utils/extractor.rs:92:54
   |
92 |         let mut archive = Archive::new(archive_path).open_for_listing()
   |                                                      ^^^^^^^^^^^^^^^^ method not found in `Archive<'_>`

error[E0599]: no method named `open_for_extraction` found for struct `unrar::Archive` in the current scope
   --> src/utils/extractor.rs:115:82
    |
115 |                     let mut extract_archive = Archive::new(archive_path.clone()).open_for_extraction()
    |                                                                                  ^^^^^^^^^^^^^^^^^^^ method not found in `Archive<'_>`
```

### 当前状态
- 文件位置：`src/utils/extractor.rs`
- 方法：`extract_rar()` 
- 当前实现：返回错误 "RAR format temporarily disabled due to API incompatibility"

## 修复需求

### 1. 检查 unrar crate 版本和文档
- 当前 Cargo.toml 中的版本：需要确认正确的 API
- 查看最新的 unrar crate 文档：https://docs.rs/unrar/

### 2. 可能的解决方案

#### 选项 A：修复当前 unrar crate 用法
```rust
// 需要研究正确的 API 调用方式
// 可能需要不同的方法名或参数
```

#### 选项 B：替换为其他 RAR 库
- 考虑使用 `rar` crate 或其他替代方案
- 评估各种库的功能和维护状态

#### 选项 C：使用系统调用
- 通过 `std::process::Command` 调用系统的 unrar 工具
- 需要确保目标系统安装了 unrar

### 3. 预期的函数签名
```rust
fn extract_rar<P: AsRef<Path>>(&self, path: P) -> Result<Vec<ExtractedFile>>
```

需要实现：
- 打开 RAR 档案
- 列出所有文件条目
- 过滤图片文件（jpg, jpeg, png, gif, webp, bmp）
- 提取文件内容到内存
- 返回 `ExtractedFile` 结构体数组

### 4. 集成要求
- 必须与现有的 `ArchiveExtractor` 结构保持兼容
- 错误处理应使用 `anyhow::Result`
- 需要支持常见的 RAR 和 CBR 文件
- 性能要求：能够处理大型档案文件

## 测试计划

修复完成后需要测试：
1. 基本 RAR 文件解压
2. CBR 格式文件（漫画专用 RAR）
3. 包含中文文件名的档案
4. 大型档案文件性能
5. 错误处理（损坏的文件、受密码保护的文件等）

## 相关文件

- `src/utils/extractor.rs` - 主要修复文件
- `Cargo.toml` - 可能需要更新依赖
- `src/services/archive_cache_service.rs` - 使用提取器的缓存服务
- Phase 2 完成的缓存系统依赖正确的 RAR 提取

## 优先级

**中等优先级** - RAR/CBR 是常见的漫画格式，但当前 ZIP/CBZ 和 7Z/CB7 支持已经覆盖了大部分用例。

## 完成标准

- [ ] RAR 和 CBR 文件能够正确提取
- [ ] 编译无错误和警告
- [ ] 通过基本功能测试
- [ ] 更新相关文档
- [ ] 考虑添加单元测试

---

**创建日期**: 2025-08-01  
**当前状态**: 待修复  
**负责人**: 手动修复需求