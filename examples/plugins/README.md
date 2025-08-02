# OtamoryX 插件开发指南

本目录包含 OtamoryX 插件系统的示例和开发指南。

## 插件系统概述

OtamoryX 插件系统是一个基于 Rust 的可扩展架构，允许开发者创建自定义功能来扩展核心系统。插件采用动态库形式，通过标准化的接口与主系统交互。

## 插件类型

### 基础插件类型
1. **元数据提取器** - 从归档文件中提取和生成元数据
2. **图像处理器** - 处理和优化图像文件
3. **内容分析器** - 分析内容并生成标签
4. **外部集成器** - 与外部服务和 API 集成

### 高级插件功能
- **自定义 API 端点** - 注册自定义 REST API 路由
- **定时任务** - 后台定时执行的任务
- **事件钩子** - 响应系统事件的回调函数
- **数据库扩展** - 自定义数据模型和查询

## 示例插件

### [metadata-extractor](./metadata-extractor/) - 元数据提取器
**功能**：从文件名、目录结构和基础属性中自动提取元数据并生成标签

**特性**：
- 可配置的正则表达式模式匹配
- 多个提取器组合使用
- 置信度评分系统
- 自动标签应用

**适用场景**：
- 大量未标记的归档文件
- 标准化的文件命名规范
- 自动化内容组织

### [image-processor](./image-processor/) - 图像处理器
**功能**：高级图像处理和优化，支持格式转换和质量控制

**特性**：
- 多种处理模式（优化、增强、调整大小）
- 并行批处理
- REST API 接口
- 实时进度监控
- 定时任务支持

**适用场景**：
- 大量图像文件优化
- 格式标准化需求
- 存储空间优化
- 图像质量提升

## 插件开发步骤

### 1. 项目初始化

```bash
# 创建插件项目
mkdir my-plugin
cd my-plugin

# 初始化 Cargo 项目
cargo init --lib
```

### 2. 配置 Cargo.toml

```toml
[package]
name = "my-plugin"
version = "1.0.0"
edition = "2021"

[dependencies]
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite"] }
tokio = { version = "1.0", features = ["full"] }

[lib]
name = "my_plugin"
crate-type = ["cdylib"]
```

### 3. 创建插件元数据 (plugin.toml)

```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "我的自定义插件"
author = "Your Name"
entry_point = "src/lib.rs"

[capabilities]
metadata_extraction = false
archive_processing = true
custom_endpoint = false
scheduled_task = false

[permissions]
network = false
filesystem_read = ["/tmp/otamoryx/*"]
database_read = true
database_write = ["archives"]
custom_endpoints = false
scheduled_tasks = false

[config_schema]
enabled = { type = "boolean", default = true }
custom_setting = { type = "string", default = "default_value" }
```

### 4. 实现插件接口

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfig {
    pub enabled: bool,
    pub custom_setting: String,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_setting: "default_value".to_string(),
        }
    }
}

pub struct MyPlugin {
    config: PluginConfig,
}

impl MyPlugin {
    pub fn new(config: PluginConfig) -> Self {
        Self { config }
    }
    
    pub async fn process_archive(&self, archive_id: &str, archive_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 实现你的处理逻辑
        println!("Processing archive: {} at {}", archive_id, archive_path);
        Ok(())
    }
}

// 插件入口点
#[no_mangle]
pub extern "C" fn plugin_init(config_json: *const i8) -> *mut MyPlugin {
    let config = if config_json.is_null() {
        PluginConfig::default()
    } else {
        let config_str = unsafe { std::ffi::CStr::from_ptr(config_json).to_str().unwrap_or("{}") };
        serde_json::from_str(config_str).unwrap_or_default()
    };

    Box::into_raw(Box::new(MyPlugin::new(config)))
}

#[no_mangle]
pub extern "C" fn plugin_cleanup(plugin: *mut MyPlugin) {
    if !plugin.is_null() {
        unsafe {
            Box::from_raw(plugin);
        }
    }
}
```

### 5. 创建构建脚本

```bash
#!/bin/bash
set -e

echo "构建插件..."
cargo build --release

mkdir -p target/plugin-package
cp plugin.toml target/plugin-package/
cp target/release/libmy_plugin.so target/plugin-package/ 2>/dev/null || \
cp target/release/libmy_plugin.dylib target/plugin-package/ 2>/dev/null || \
cp target/release/my_plugin.dll target/plugin-package/

cd target/plugin-package
tar -czf ../my-plugin-1.0.0.tar.gz *
cd ../..

echo "插件包已创建: target/my-plugin-1.0.0.tar.gz"
```

## 插件接口规范

### 核心 Trait

```rust
#[async_trait]
pub trait Plugin {
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn get_info(&self) -> PluginInfo;
}

#[async_trait]
pub trait ArchiveProcessor {
    async fn process_archive(&self, archive_id: &str, archive_path: &str) -> Result<ProcessingResult, Box<dyn std::error::Error>>;
}

#[async_trait]
pub trait MetadataExtractor {
    async fn extract_metadata(&self, archive_path: &str) -> Result<ExtractedMetadata, Box<dyn std::error::Error>>;
}
```

### 配置系统

插件配置通过 `plugin.toml` 文件定义，支持以下数据类型：
- `boolean` - 布尔值
- `string` - 字符串
- `number` - 数字
- `array` - 数组
- `object` - 对象

### 权限系统

插件需要声明所需的权限：
- `network` - 网络访问权限
- `filesystem_read` - 文件系统读取权限
- `filesystem_write` - 文件系统写入权限
- `database_read` - 数据库读取权限
- `database_write` - 数据库写入权限
- `custom_endpoints` - 自定义 API 端点权限
- `scheduled_tasks` - 定时任务权限

## 最佳实践

### 错误处理
- 使用适当的错误类型和消息
- 实现优雅的失败恢复
- 记录详细的错误日志

### 性能优化
- 避免阻塞主线程
- 使用异步操作处理 I/O
- 合理使用缓存机制
- 注意内存使用和清理

### 安全考虑
- 验证所有输入数据
- 遵循最小权限原则
- 避免执行不安全的操作
- 正确处理敏感信息

### 配置管理
- 提供合理的默认值
- 验证配置的有效性
- 支持配置热重载
- 文档化所有配置选项

## 调试和测试

### 本地开发
1. 使用本地路径加载方式进行开发
2. 启用调试日志输出
3. 使用单元测试验证功能

### 集成测试
1. 在测试环境中安装插件
2. 验证与主系统的集成
3. 测试各种边界条件

### 性能测试
1. 使用大量数据测试性能
2. 监控内存和 CPU 使用
3. 验证并发处理能力

## 发布和分发

### 版本控制
- 遵循语义化版本规范
- 维护详细的变更日志
- 支持向后兼容性

### 文档要求
- 完整的 README 文档
- API 接口文档
- 配置选项说明
- 使用示例和最佳实践

### 插件包格式
插件包应包含：
- `plugin.toml` - 插件元数据
- 编译后的动态库文件
- `README.md` - 文档
- 其他依赖文件

## 社区和支持

### 开发者资源
- [官方文档](https://github.com/your-org/otamoryx/docs)
- [API 参考](https://github.com/your-org/otamoryx/api)
- [示例仓库](https://github.com/your-org/otamoryx-plugins)

### 获取帮助
- GitHub Issues
- 开发者论坛
- 技术支持邮箱

### 贡献指南
欢迎贡献新的插件示例和改进现有文档。请遵循项目的贡献指南和代码规范。

---

更多详细信息请参考各个示例插件的具体文档和实现代码。