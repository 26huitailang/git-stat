# git-stat

- 克隆repo，可能有多个，放到一个目录下`./repos`
- 指定repo分支
- 统计所有commit信息
  - 按作者分类
  - 统计单个commit：
    - 插入
    - 删除
    - 时间，后面用于过滤
- 输出csv
- 支持tui直接打印（后续支持）
  - graph
  - table

```yml
output: [csv, tui]
repos:
  - url: https://github.com/xxx/xxx.git
    branchs: [master, dev]
    authors: [xxx, xxx]
```

```shell
./git-stat --url https://github.com/xxx/xxx.git --branch master --output-csv xxx.csv --output-tui
```
