---
name: ledata-python-interface
description: 用 ledata_py / ledata-cli 提取与导出 LED 器件测试数据（xlsx）。当需要在本项目中用 Python 或命令行读取器件性能数据、按电压提取光谱、导出 Origin 口径 TSV、或调用 LEData 的数据口径做自动化时使用。也用于理解用户的 LED 器件分析需求（片子/器件点/批次/EQE/光谱等术语）并回答 LEData 桌面应用的使用问题。
---

# LEData 数据接口（LED 器件测试数据分析）

## 这是什么，用户是谁

- **LEData**：LED 器件测试数据工具，包含桌面可视化 App（Tauri GUI）与 Python/CLI 数据接口，二者使用**同一份 Rust 核心**，数据口径完全一致。
- **数据来源**：犀谱光电 XPQY-EQE 测试设备客户端导出的文件（xlsx 数据表格，部分新格式配套 `VASpectrum` 光谱 CSV）。
- **用户**：LED/OLED 研究人员，做器件表征数据分析，常用 Origin/Excel 与科研绘图工具。
- **一句话**：用户加载一批测试文件后，可以看器件点分布性能图（Mapping）、画电流密度-电压/亮度-电压/EQE-亮度曲线、按电压查看发光光谱、把曲线数据导出成 Origin 可直接粘贴的表格。

## 术语映射（用户的话 → 本项目概念）

| 用户可能说 | 本项目概念 | 接口对象 |
| --- | --- | --- |
| 片子 / 芯片 / 文件 / 样品片 | Chip（一次测试的玻片，8 个器件槽位 A–H） | `parse_file` 返回的 `Chip`；目录解析结果里的 `chips[i]` |
| 点 / 器件 / Site / 像素 / 单元 | Device（片上的器件点） | `chip.devices[slot]`；名字形如 `S1A@chip`（S+槽位号+位点字母） |
| 这一批 / 一批片子 / 目录 | 数据目录（同批制备+测试的多个片子，每片一个文件） | `parse_dir` → `DirResult.chips` |
| 亮度 / L / cd/m² | Luminance（红外器件为 Radiance 辐照度） | `d.luminance`、`d.max_lumi` |
| 电流密度 / J | J (mA/cm²) | `d.j`、`d.max_j` |
| 电压 / U / V | 驱动电压扫描点 | `d.u`；`spectrum_at(3.5)` 的参数就是电压 |
| EQE / 外量子效率 / 效率 | EQE (%) | `d.eqe`、`d.max_eqe` |
| 有效 EQE | 亮度 ≥ 10% 最大亮度区间内的最大 EQE | `d.valid_eqe` |
| 漏电流 / 漏电 | 开压前 J 均值（**口径可能不准**，向用户转述时要提醒） | `d.leak_j` |
| 光谱 / EL 光谱 / 发光光谱 | 某电压下的波长-强度分布（655 个波长点） | `d.spectra`、`spectrum_at(v)`、`spectra_csv` |
| 表格 / 数据文件 / 测试文件 | 设备导出的 xlsx（+ 配套光谱 CSV） | `parse_file(path)` |

GUI 玻片图上的 8 个位置 A–H 对应 `devices` 的 8 个槽位（0–7），空槽位为 `null`。

## 桌面 App（GUI）有什么

用户问"软件怎么用"时可参考：

- **打开数据**：工具栏"打开文件"（单个 xlsx）或"打开文件夹"（整个目录批量加载）。
- **器件性能 Mapping**：左侧玻片图按所选模式给器件点着色（越红值越大），模式有：最大亮度/辐照度、最大 EQE、有效 EQE、最大电流密度、平均漏电流（不准，仅供参考）；右键器件点/玻片可排除异常点。
- **绘制曲线**：左键点选器件点（或玻片全选）加入右侧绘图；三张图——J/亮度-电压（双 Y 轴）、EQE-亮度（可切对数轴）、光谱（滑块选电压）。
- **导出**："复制性能数据/复制光谱数据"到剪贴板（制表符分隔，Origin/Excel 直接粘贴）；"导出图片"（PNG，仅供预览）。

## Python/CLI 接口能力（大白话 + 例句）

接口定位：批量、可编程地拿到与 GUI 完全相同口径的数据。**默认输出摘要（无原始数组）**，避免撑爆 Agent 上下文；需要原始曲线时再显式取。

| 用户原话（例） | 应该做什么 |
| --- | --- |
| "这批片子里哪个点 EQE 最高？" | `parse_dir` + 对 `chip.summary()` 的 `max_eqe` 排序 |
| "看看 S2 漏电怎么样 / 哪个点漏电大" | summary 的 `leak_j` 排序（提醒口径可能不准） |
| "提取 3.5V 下片子 c1 的 S2 光谱" | `parse_file` + `chip.spectra_at(3.5, site="S2@c1")` |
| "把曲线数据导出成 Origin 能贴的表" | `performance_csv(devices)` / `spectra_csv(devices, u_index)` |
| "3.5V 和 4V 的光谱有啥区别" | 两次 `spectrum_at`，对比 `values` |
| "画个 J-V 曲线" | 取 `d.u`/`d.j` 数组后由你（Agent）用 matplotlib 自绘 |
| "哪个点失效了/数据不完整" | summary 里 `n_points` 异常、槽位为 `null` 的器件 |

环境与 API 细节见下；CLI 版本把上面每件事做成子命令式参数（`ledata-cli <路径> [--voltage V] [--site 1A] [--full] [--csv performance|spectra]`），适合没有 Python 环境时子进程调用。

## 环境

### 普通用户（安装版，路径含占位符，按用户实际安装目录替换）

```python
import sys
sys.path.insert(0, r"<LEDATA_INSTALL_DIR>\py-interface")
import ledata_py as lp
```

- CLI：`<LEDATA_INSTALL_DIR>\cli\ledata-cli.exe`
- 需自备 Python ≥ 3.10 与 numpy；必须 `sys.path.insert(0, ...)`，验证 `lp.__file__` 指向安装目录。

### 开发者（本仓库）

```shell
# 仓库根目录
uv venv .venv && source .venv/Scripts/activate
uv pip install maturin pytest numpy
cd crates/ledata-py && maturin develop --release
```

绑定更新必须重新 `maturin develop`（editable 安装会遮蔽手动拷贝的 pyd）。

## 典型任务

### 提取性能数据（Agent 场景，先读摘要）

```python
import ledata_py as lp
chip, warning = lp.parse_file(path)     # warning 文案与 GUI 弹窗一致，非 None 时如实转述
summary = chip.summary()                # 五指标 + n_points/u_min/u_max，无原始数组
```

- 不要直接 `chip.to_json()` 或访问 `d.spectra`（全量矩阵很大，吃上下文）；摘要不够用时再按需取数。

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
- 目录里混入无关命名/损坏的 xlsx 会作为 per-file 错误返回（GUI 弹窗、
  `DirResult.errors`、CLI `errors` 字段），其余文件正常解析，不是整体失败。
- 真实样例在 `.session/fixtures/`（快捷方式，路径不入库）；pytest 真实用例
  路径不可达时自动 skip。
- 三方对拍（改导出口径必跑）：`.venv/Scripts/python tests/golden/run_golden.py`。
- 版本发布前六处同步：package.json / src-tauri/tauri.conf.json /
  src-tauri/Cargo.toml / crates/{ledata-core,ledata-py,ledata-cli}/Cargo.toml。
- 分发：release 分支 push 或 workflow_dispatch 触发；py-interface、CLI 与
  本 SKILL 文件随安装包落 `<安装目录>\py-interface\`、`<安装目录>\cli\`、
  `<安装目录>\skill\`。
