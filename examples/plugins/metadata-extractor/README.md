# 元数据提取器插件

这是一个用于从漫画归档文件中自动提取元数据并生成标签的插件。

## 功能特性

- **文件名解析**：从文件名中提取作者、系列、语言等信息
- **目录结构分析**：从文件路径中推断分类和系列信息
- **基础标签生成**：自动识别语言、格式、分辨率等基础标签
- **可配置的提取规则**：支持自定义正则表达式模式
- **置信度评估**：对提取结果进行可信度评分

## 配置选项

```json
{
  "enabled_extractors": ["filename", "directory", "basic_tags"],
  "tag_patterns": {
    "artist": "\\[([^\\]]+)\\]",
    "series": "^([^(\\[]+)",
    "language": "\\b(chinese|english|japanese|korean)\\b"
  },
  "auto_tag_threshold": 0.8
}
```

### 配置说明

- `enabled_extractors`: 启用的提取器列表
  - `filename`: 文件名提取器
  - `directory`: 目录结构提取器  
  - `basic_tags`: 基础标签提取器

- `tag_patterns`: 自定义标签提取的正则表达式模式
  - `artist`: 提取作者信息的模式
  - `series`: 提取系列信息的模式
  - `language`: 提取语言信息的模式

- `auto_tag_threshold`: 自动应用标签的最低置信度阈值 (0.0-1.0)

## 提取规则示例

### 文件名模式
- `[Author Name] Series Title (Chapter 01) [Chinese].zip`
  - 作者: Author Name
  - 系列: Series Title
  - 语言: Chinese

### 目录结构模式
- `/manga/One Piece/Chapter 001.zip`
  - 系列: One Piece
  - 类型: manga

### 基础标签模式
- 语言检测: `chinese`, `english`, `japanese`, `korean`
- 分辨率检测: `1920x1080`, `4k`, `hd`
- 格式检测: 文件扩展名

## 构建和安装

1. 构建插件：
   ```bash
   ./build.sh
   ```

2. 安装插件：
   - 在 OtamoryX 管理界面中选择"插件管理"
   - 点击"安装插件"
   - 上传生成的 `target/metadata-extractor-1.0.0.tar.gz` 文件

3. 配置插件：
   - 启用插件
   - 根据需要调整配置参数
   - 保存配置

## 开发说明

### 文件结构
```
metadata-extractor/
├── plugin.toml          # 插件元数据和配置
├── Cargo.toml          # Rust 项目配置
├── src/
│   └── lib.rs          # 主要实现代码
├── build.sh            # 构建脚本
└── README.md           # 说明文档
```

### 核心组件

1. **MetadataExtractorPlugin**: 主插件类，协调各个提取器
2. **FilenameExtractor**: 文件名解析提取器
3. **DirectoryExtractor**: 目录结构分析提取器
4. **BasicTagsExtractor**: 基础标签识别提取器

### 扩展开发

要添加新的提取器，需要：

1. 实现 `MetadataExtractor` trait
2. 在 `MetadataExtractorPlugin` 中集成新提取器
3. 更新配置架构以支持新提取器选项

## 性能考虑

- 插件使用正则表达式进行模式匹配，对于大量文件处理时请注意性能
- 置信度计算基于提取信息的数量和质量
- 数据库操作使用事务确保数据一致性

## 许可证

本插件遵循 GNU 通用公共许可证 v3.0。