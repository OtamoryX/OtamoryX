#!/bin/bash

# 构建图像处理器插件

set -e

echo "构建图像处理器插件..."

# 构建动态库
cargo build --release

# 创建插件包目录
mkdir -p target/plugin-package

# 复制必要文件
cp plugin.toml target/plugin-package/
cp target/release/libimage_processor.so target/plugin-package/ 2>/dev/null || cp target/release/libimage_processor.dylib target/plugin-package/ 2>/dev/null || cp target/release/image_processor.dll target/plugin-package/
cp README.md target/plugin-package/ 2>/dev/null || echo "# Image Processor Plugin" > target/plugin-package/README.md

# 创建插件包
cd target/plugin-package
tar -czf ../image-processor-1.0.0.tar.gz *
cd ../..

echo "插件包已创建: target/image-processor-1.0.0.tar.gz"
echo "可以通过 OtamoryX 管理界面上传此文件来安装插件"