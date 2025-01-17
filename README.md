# git-stat

## Introduction

![polars](./docs/images/polars.png)
![ratatui](./docs/images/tui.png)

![](./design.excalidraw.png)

## Usage

working on demo ...

```shell
# 统计配置中所有仓库的代码变更
RUST_LOG=trace git-stat --format table --since 2024-01-01 --until 2024-03-31
```

```mermaid
flowchart TD

Start --> source{input --source} --> |Y|detail.csv
source --> |N|config{.git-stat.yml}
config --> |Y|parse --> R1(repos) -->|parallel| R2(commit collect)
R2 --> commit-list(commit info) --> detail.csv --> polars
config --> |N|generate? --> config
polars(polars calc) --> sqlparse? --> summary --> output --> |csv|csv
output --> |polar|terminal
output --> |table|tui
tui-->filter(input condition) --> polars
```

## features

- 并行计算统计
- 克隆repo，可能有多个，放到一个目录下`./repos`
- 指定repo分支
- 统计所有commit信息
  - 路径过滤支持(pathspec fnmatch语法)
  - 单个commit：
    - repo
    - date
    - branch
    - commit_id
    - author
    - insertions
    - deletions
    - message
  - 统计
    - 作者alias聚合: config authors定义
    - insertions/deletions sum
    - 时间过滤: --since --until
    - 输出一份detail文件作为过程
  - 支持MR识别（这部分代码不应统计，如果一个commit parent_count > 1，则应该是合并commit）
  - [ ] polars 加载和计算detail.csv：交互式的，下面展示结果，上面input输入信息
- output
  - csv
  - tui(ratatui)
    - table
    - [ ] graph

## 交叉编译

要在MacOS上使用Cargo交叉编译Windows程序，您需要确保已经安装了对应的目标工具链，并在构建时指定正确的目标平台。以下是详细步骤：

### 安装Windows目标工具链

首先，确保你已经安装了`rustup`。然后，安装适用于Windows的工具链。对于64位Windows（最常见的情况），你需要安装`x86_64-pc-windows-gnu`或`x86_64-pc-windows-msvc`工具链。这里我们以`x86_64-pc-windows-gnu`为例，因为它不需要Windows SDK并且与MinGW兼容：

```sh
rustup target add x86_64-pc-windows-gnu
```

### 配置Cargo

对于简单的项目，直接在Cargo命令中指定目标平台即可。如果你的项目依赖于特定的Windows库或者需要更复杂的配置，可能需要在`Cargo.toml`中设置特定的属性或依赖项。

### 交叉编译

现在，你可以使用以下命令来交叉编译你的Rust项目为Windows可执行文件：

```sh
# brew install mingw-w64  // macos安装
cargo build --target=x86_64-pc-windows-gnu
```

这将在`target/x86_64-pc-windows-gnu/debug/`目录下生成一个Windows可执行文件。

### 注意事项

- **链接器**: 使用`x86_64-pc-windows-gnu`工具链时，Cargo默认会尝试使用MinGW-w64提供的链接器。如果遇到找不到链接器的问题，你可能需要安装MinGW-w64或确保其路径被正确配置到系统PATH中。
- **DLLs**: 如果你的程序依赖于动态库（DLLs），在Windows上运行时需要确保这些DLLs可用。交叉编译不会生成这些DLLs，你可能需要从适当的地方获取它们。
- **UI框架**: 如果你的程序使用了特定于操作系统的UI框架（如Windows的winapi），请确保你的代码适当地处理了平台差异。

通过上述步骤，你就可以在MacOS上成功地交叉编译出适用于Windows的Rust程序了。