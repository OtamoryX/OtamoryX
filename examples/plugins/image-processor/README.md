# 图像处理器插件

> **注意**: 这是一个高级插件示例，展示了复杂的功能实现。由于依赖较多的外部库和数据库集成，可能需要额外配置才能编译。如果你是初学者，建议先从 `metadata-extractor` 示例开始学习。

这是一个高级图像处理插件，用于优化和增强漫画页面图像，支持批量处理、格式转换和质量控制。

## 功能特性

- **多模式处理**：优化、增强、调整大小等多种处理模式
- **格式转换**：支持 WebP、JPEG、PNG 等现代图像格式
- **批量处理**：高效的并行处理大量图像文件
- **质量控制**：可配置的压缩质量和文件大小限制
- **图像增强**：锐化、降噪、对比度调整等功能
- **REST API**：提供完整的 API 接口用于处理控制
- **定时任务**：支持自动批量处理新归档
- **进度监控**：实时处理进度跟踪和状态管理

## 配置选项

```json
{
  "processing_modes": ["optimize", "enhance"],
  "output_format": "webp",
  "quality_settings": {
    "webp_quality": 85,
    "jpeg_quality": 90,
    "png_compression": 6,
    "max_width": 2048,
    "max_height": 2048
  },
  "batch_size": 10,
  "auto_process": false,
  "enhancement_settings": {
    "sharpen": true,
    "noise_reduction": true,
    "contrast_adjust": true,
    "brightness_adjust": false
  }
}
```

### 配置说明

#### 处理模式 (processing_modes)
- `optimize`: 压缩优化（调整尺寸、格式转换）
- `enhance`: 图像增强（锐化、降噪、对比度）
- `resize`: 尺寸调整（强制调整到最大尺寸限制）

#### 输出格式 (output_format)
- `webp`: WebP 格式（推荐，体积小质量高）
- `jpeg`: JPEG 格式（兼容性好）
- `png`: PNG 格式（无损压缩）

#### 质量设置 (quality_settings)
- `webp_quality`: WebP 压缩质量 (0-100)
- `jpeg_quality`: JPEG 压缩质量 (0-100)
- `png_compression`: PNG 压缩级别 (0-9)
- `max_width/max_height`: 图像最大尺寸限制

#### 批处理设置
- `batch_size`: 单次处理的归档数量
- `auto_process`: 是否自动处理新归档

#### 增强设置 (enhancement_settings)
- `sharpen`: 图像锐化
- `noise_reduction`: 降噪处理
- `contrast_adjust`: 对比度调整
- `brightness_adjust`: 亮度调整

## API 接口

插件提供以下 REST API 端点：

### 处理操作
- `POST /api/v1/plugins/image-processor/process/:archive_id` - 处理指定归档
- `GET /api/v1/plugins/image-processor/jobs` - 获取处理任务列表
- `GET /api/v1/plugins/image-processor/jobs/:job_id` - 获取任务详情
- `POST /api/v1/plugins/image-processor/jobs/:job_id/cancel` - 取消处理任务

### 请求参数
```json
{
  "force_reprocess": false  // 是否强制重新处理已处理的归档
}
```

### 响应示例
```json
{
  "original_size": 52428800,
  "processed_size": 31457280,
  "compression_ratio": 40.0,
  "processing_time_ms": 15000,
  "pages_processed": 24,
  "pages_failed": 0
}
```

## 处理流程

1. **归档提取**：临时提取归档文件到工作目录
2. **图像发现**：扫描支持的图像格式（JPG、PNG、WebP、BMP、TIFF）
3. **并行处理**：使用 Rayon 并行处理所有图像文件
4. **格式转换**：根据配置转换到目标格式
5. **质量优化**：应用压缩和优化设置
6. **结果统计**：计算压缩比例和处理时间

## 性能特性

- **并行处理**：利用多核 CPU 进行并行图像处理
- **内存优化**：流式处理避免大量内存占用
- **进度跟踪**：实时更新处理进度到数据库
- **错误处理**：单个文件失败不影响整体处理
- **资源管理**：自动清理临时文件和内存

## 构建和安装

1. 安装依赖：
   ```bash
   # Ubuntu/Debian
   sudo apt-get install build-essential pkg-config
   
   # macOS
   brew install pkg-config
   ```

2. 构建插件：
   ```bash
   ./build.sh
   ```

3. 安装插件：
   - 在 OtamoryX 管理界面中选择"插件管理"
   - 点击"安装插件"
   - 上传生成的 `target/image-processor-1.0.0.tar.gz` 文件

4. 配置插件：
   - 启用插件
   - 根据需要调整处理模式和质量设置
   - 保存配置

## 使用建议

### 针对不同内容的优化建议

**彩色漫画**：
- 使用 WebP 格式，质量设置 80-90
- 启用锐化和对比度调整
- 最大尺寸设置为 1920x1080

**黑白漫画**：
- 使用 PNG 格式保持清晰度
- 启用锐化，关闭色彩调整
- 可以设置更高的压缩级别

**高质量原图**：
- 仅使用优化模式，关闭增强功能
- 设置较高的质量参数
- 保持原始尺寸比例

### 性能调优

- **内存限制环境**：减小 batch_size，关闭自动处理
- **高性能环境**：增大 batch_size，启用所有处理模式
- **存储敏感**：选择 WebP 格式，降低质量设置

## 故障排除

### 常见问题

1. **处理失败**：检查图像文件格式和权限
2. **内存不足**：减小批处理大小或图像尺寸限制
3. **处理缓慢**：调整并行度或简化处理流程

### 日志查看
插件处理日志会记录到系统日志中，可以通过以下方式查看：
- 系统健康监控页面
- 插件管理界面的处理任务列表
- 服务器日志文件

## 扩展开发

### 添加新的处理模式

1. 在 `process_single_image` 函数中添加新的处理逻辑
2. 在配置架构中声明新的模式选项
3. 更新文档和示例配置

### 自定义增强算法

1. 在 `enhance_image` 函数中实现新算法
2. 添加相应的配置选项
3. 考虑性能影响和内存使用

## 许可证

本插件遵循 GNU 通用公共许可证 v3.0。