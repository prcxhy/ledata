# ledata_py — Agent 向使用文档

`ledata_py` 是 LEData 的 Python 绑定：LED 器件测试数据（xlsx）提取与消费。
**与 LEData GUI 使用同一份 Rust 核心（ledata-core），数据口径完全一致。**

## 安装/构建（开发环境）

```shell
# 仓库根目录
uv venv .venv && source .venv/Scripts/activate
uv pip install maturin pytest numpy
cd crates/ledata-py && maturin develop --release
```

更新绑定后必须重新 `maturin develop`（editable 安装会遮蔽手动拷贝的 pyd）。

## API 一览

```python
import ledata_py as lp

# —— 解析（全量读取，与 GUI 一致）——
chip, warning = lp.parse_file(r"D:\data\20260703 101945,Data,02.xlsx")
#   warning: str | None，文案与 GUI 弹窗一致（"… 光谱文件缺失 / 光谱文件不匹配"）
result = lp.parse_dir(r"D:\data")
result.chips; result.warnings; result.errors

# —— 数据对象 ——
chip.name                    # "02"
chip.devices                 # list[DeviceData | None]，固定 8 槽位（GUI 玻片 A-H）
d = chip.devices[1]          # 空位为 None
d.name                       # "S2@02"
d.is_vis                     # 可见光（True）/ 红外（False）
d.slot                       # 槽位号 0-7
d.u; d.j; d.luminance; d.eqe # np.ndarray[float64]，每次访问都拷贝
d.wavelength                 # vis 已按 GUI 口径裁剪为 655 点
d.spectra                    # 2D (n_u, n_wavelength)，行 = 电压索引
d.max_j; d.max_lumi; d.max_eqe; d.valid_eqe; d.leak_j
d.to_json()                  # 全量 JSON，字段与 GUI JSON 契约一致
d.to_dict(); chip.to_json()

# —— Agent 摘要（默认无原始数组，保护上下文）——
chip.summary()               # dict，与 CLI 输出同构
# { name, devices: [ { slot, name, is_vis, max_j, max_lumi, max_eqe,
#   valid_eqe, leak_j, n_points, u_min, u_max, spectra: None } | None ] }
chip.summary(voltage=3.5)    # 附加 spectra: { requested_voltage, matched_voltage, matched_index }

# —— 按电压消费光谱（解析全量，匹配在消费层）——
pt = d.spectrum_at(3.5)      # Device 级 → SpectrumPoint | None
pts = chip.spectra_at(3.5)           # Chip 级：所有有数据器件各自匹配
pts = chip.spectra_at(3.5, site="2") # site 过滤（"2" / "S2@02" 均可，宽松匹配）
pt.requested_voltage; pt.matched_voltage; pt.matched_index
pt.wavelength; pt.values             # np.ndarray

# —— 导出（与 GUI"复制性能/光谱数据"逐字符同口径，Origin/Excel 直贴）——
lp.performance_csv([d for d in chip.devices if d]) -> str
lp.spectra_csv(devices, u_index) -> str

# —— 异常族（中文文案沿用 GUI）——
lp.LeDataError
├─ lp.UnsupportedFormatError   # "<file> 表格的格式不受支持"
├─ lp.EmptyDirError            # "该目录为空"
├─ lp.NoSupportedDataError     # "该目录下没有找到符合支持格式的数据表格"
└─ lp.InvalidPathError         # 路径不存在 / 文件目录类型不符（绑定层前置校验）
```

## 行为说明

- **光谱**：解析始终全量读入（与 GUI 一致）；"不指定就不提取"指输出/导出层
  不含光谱段。`spectrum_at` 无光谱数据（warning 场景）时返回 None。
- **电压匹配**：器件在各自电压列取 `argmin(|u - V|)`，并列取先出现者。
- **warning 是忠实行为**：真实样例中光谱行数与扫描点数不符时会得到
  "光谱文件不匹配"，与 GUI 弹窗一致，性能数据不受影响。
- **边缘数据可能 panic**：core 沿用 GUI 的 `unwrap` 风格（登记 ISSUES #1），
  传入前自行保证路径存在、文件为受支持格式。

## 相关工具

- CLI：`crates/ledata-cli`（`ledata-cli <path> --voltage --site --full --csv`），
  输出与 `chip.summary()` 同构，退出码 0/1/2。
- 三方对拍（TS↔Rust↔Python 导出口径）：`.venv/Scripts/python tests/golden/run_golden.py`
