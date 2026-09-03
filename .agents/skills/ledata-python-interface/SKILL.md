---
name: ledata-python-interface
description: 用 ledata_py / ledata-cli 提取与导出 LED 器件测试数据（xlsx）。当需要在本项目中用 Python 或命令行读取器件性能数据、按电压提取光谱、导出 Origin 口径 TSV、或调用 LEData 的数据口径做自动化时使用。
---

# LEData Python/CLI 数据接口

LEData GUI、`ledata_py`（Python）、`ledata-cli` 共用同一份 Rust 核心 `ledata-core`，数据口径完全一致。

## 环境

- 开发环境 venv 在仓库根 `.venv/`（`uv venv` 创建）；绑定更新必须走
  `cd crates/ledata-py && source ../../.venv/Scripts/activate && maturin develop --release`。
- 完整 API 与行为说明：`docs/ledata_py.md`（Agent 向文档，先读它）。

## 典型任务

### 提取性能数据（Agent 场景，先读摘要）

```python
import ledata_py as lp
chip, warning = lp.parse_file(path)     # warning 文案与 GUI 弹窗一致，非 None 时如实转述
summary = chip.summary()                # 五指标 + n_points/u_min/u_max，无原始数组
```

- 不要直接 `chip.to_json()` 或访问 `d.spectra`（全量矩阵很大，吃上下文）；
  摘要不够用时再按需取数。

### 按电压提取光谱

```python
pt = chip.devices[1].spectrum_at(3.5)       # 单器件
pts = chip.spectra_at(3.5)                  # 整片（各器件各自匹配）
pts = chip.spectra_at(3.5, site="2")        # site 过滤，"2"/"S2@02" 均可
```

- 匹配规则：各自电压列 `argmin(|u − V|)`，并列取先出现者；
- 无光谱数据（warning 场景）时返回 None；
- 解析始终全量，"不提取"只发生在输出/消费层。

### 导出 Origin 口径 TSV

```python
devices = [d for d in chip.devices if d]
tsv = lp.performance_csv(devices)             # 与 GUI"复制性能数据"逐字符一致
tsv = lp.spectra_csv(devices, u_index)        # 与 GUI"复制光谱数据"一致
```

### 命令行（子进程场景）

```
ledata-cli <path>                        # 摘要 JSON（与 chip.summary() 同构）
ledata-cli <path> --voltage 3.5 --site 1A
ledata-cli <path> --full                 # 全量（大！确认需要再用）
ledata-cli <path> --csv performance      # GUI 同口径 TSV 到 stdout
```

退出码：0 成功 / 1 数据类失败 / 2 用法错误。

## 注意

- 异常族：`LeDataError` ← `UnsupportedFormatError` / `EmptyDirError` /
  `NoSupportedDataError` / `InvalidPathError`（中文文案，与 GUI 一致）。
- 真实样例在 `.session/fixtures/`（快捷方式，路径不入库）；pytest 真实用例
  路径不可达时自动 skip。
- 三方对拍（改导出口径必跑）：`.venv/Scripts/python tests/golden/run_golden.py`。
- 版本发布前六处同步：package.json / src-tauri/tauri.conf.json /
  src-tauri/Cargo.toml / crates/{ledata-core,ledata-py,ledata-cli}/Cargo.toml。
- 分发：release 分支 push 或 workflow_dispatch 触发；py-interface 与 CLI 随
  安装包落 `<安装目录>\py-interface\`、`<安装目录>\cli\`。
