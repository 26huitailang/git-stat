# Makefile 示例，用于Rust项目

# 设置Rust工具链路径
RUSTC ?= rustc
CARGO ?= cargo

# 编译目标
TARGET = my_rust_project

# 编译标志
FLAGS = -C opt-level=3 --edition=2018

# 默认任务：编译并运行项目
all: build run

# 使用Cargo构建项目
build: test
	$(CARGO) build $(FLAGS)

build-windows: test
	$(CARGO) build --target=x86_64-pc-windows-gnu

# 运行项目
run:
	$(CARGO) run

# 清理编译生成的文件
clean:
	$(CARGO) clean
	rm -rf target

# 格式化代码
fmt:
	$(CARGO) fmt

# 执行单元测试
test:
	$(CARGO) test

# 构建并打包为release版本
release:
	$(CARGO) build --release

# 安装项目到本地cargo目录
install:
	$(CARGO) install

# 卸载项目
uninstall:
	$(CARGO) uninstall $(TARGET)

# 显示帮助信息
help:
	@echo "Makefile commands:"
	@echo "  all       - Build and run the project"
	@echo "  build     - Build the project with Cargo"
	@echo "  run       - Run the project"
	@echo "  clean     - Clean up build artifacts"
	@echo "  fmt       - Format the code with rustfmt"
	@echo "  test      - Run unit tests"
	@echo "  release   - Build a release version of the project"
	@echo "  install   - Install the project locally"
	@echo "  uninstall - Uninstall the project"
	@echo "  help      - Show this help message"

.PHONY: all build run clean fmt test release install uninstall help
