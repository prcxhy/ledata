"""ledata_py 绑定测试：异常族 / 合成路径校验 / 真实样例（skipif）。"""
import json
import pathlib

import numpy as np
import pytest

import ledata_py as lp

GUI_CONTRACT_KEYS = [
    "name", "is_vis", "u", "j", "luminance", "eqe", "wavelength", "spectra",
    "max_j", "max_lumi", "max_eqe", "valid_eqe", "leak_j",
]


# ---------- 异常族 ----------

def test_exception_hierarchy():
    assert issubclass(lp.UnsupportedFormatError, lp.LeDataError)
    assert issubclass(lp.EmptyDirError, lp.LeDataError)
    assert issubclass(lp.NoSupportedDataError, lp.LeDataError)
    assert issubclass(lp.InvalidPathError, lp.LeDataError)
    assert issubclass(lp.LeDataError, Exception)


def test_parse_file_invalid_path(tmp_path):
    with pytest.raises(lp.InvalidPathError, match="路径不存在"):
        lp.parse_file(str(tmp_path / "不存在.xlsx"))
    with pytest.raises(lp.InvalidPathError, match="路径是目录"):
        lp.parse_file(str(tmp_path))


def test_parse_dir_invalid_path(tmp_path):
    with pytest.raises(lp.InvalidPathError, match="路径不存在"):
        lp.parse_dir(str(tmp_path / "不存在"))
    (tmp_path / "a.txt").write_text("x", encoding="utf-8")
    # 忠实行为：目录内没有 .xlsx 即"该目录为空"
    with pytest.raises(lp.EmptyDirError):
        lp.parse_dir(str(tmp_path))


# ---------- 真实样例：新格式 ----------

def clean_chip(new_format_dir):
    """样例目录中第一个无 warning（光谱匹配成功）的 chip"""
    for xlsx in sorted(pathlib.Path(new_format_dir).glob("*Data*.xlsx")):
        chip, warning = lp.parse_file(str(xlsx))
        if warning is None:
            return xlsx, chip
    pytest.skip("样例中没有光谱匹配成功的文件")


def test_new_format_parse_file(new_format_dir):
    xlsx, chip = clean_chip(new_format_dir)
    assert isinstance(chip, lp.Chip)

    devices = chip.devices
    assert len(devices) == 8
    present = [d for d in devices if d is not None]
    assert len(present) >= 1

    d = present[0]
    assert d.name.startswith("S") and "@" in d.name
    assert d.is_vis is True
    assert isinstance(d.u, np.ndarray) and d.u.dtype == np.float64
    assert d.u.ndim == 1
    assert d.j.shape == d.u.shape
    assert d.luminance.shape == d.u.shape
    assert d.eqe.shape == d.u.shape
    # vis 波长窗口 [115:770] → 655 点
    assert d.wavelength.shape == (655,)
    assert d.spectra.shape == (len(d.u), 655)
    # 摘要指标与数组一致
    assert d.max_j == pytest.approx(float(d.j.max()))
    assert d.max_lumi == pytest.approx(float(d.luminance.max()))
    assert d.max_eqe == pytest.approx(float(d.eqe.max()))


def test_new_format_spectra_mismatch_warning(new_format_dir):
    """真实样例：光谱行数与扫描点数不符时，warning 文案与 GUI 一致、spectra 置空"""
    for xlsx in sorted(pathlib.Path(new_format_dir).glob("*Data*.xlsx")):
        chip, warning = lp.parse_file(str(xlsx))
        if warning is not None:
            assert warning.endswith("光谱文件不匹配") or warning.endswith("光谱文件缺失")
            assert warning.startswith(xlsx.stem)
            d0 = [d for d in chip.devices if d is not None][0]
            assert d0.wavelength.shape == (0,)
            assert d0.spectra.shape[1] == 0
            # 性能数据不受影响
            assert len(d0.u) > 0
            return
    pytest.skip("样例中没有光谱不匹配的文件")


def test_new_format_json_contract(new_format_dir):
    _, chip = clean_chip(new_format_dir)
    payload = json.loads(chip.to_json())
    assert set(payload.keys()) == {"name", "devices"}
    for d in payload["devices"]:
        if d is None:
            continue
        assert set(d.keys()) == set(GUI_CONTRACT_KEYS)
        break
    device = [d for d in payload["devices"] if d is not None][0]
    d0 = [d for d in chip.devices if d is not None][0]
    assert device["max_lumi"] == d0.max_lumi
    assert len(device["u"]) == len(d0.u)


def test_new_format_summary_and_spectrum_at(new_format_dir):
    _, chip = clean_chip(new_format_dir)
    d0 = [d for d in chip.devices if d is not None][0]

    summary = chip.summary()
    assert summary["name"] == chip.name
    assert len(summary["devices"]) == 8
    entry = [e for e in summary["devices"] if e is not None][0]
    assert entry["name"] == d0.name
    assert entry["n_points"] == len(d0.u)
    assert entry["spectra"] is None  # 未请求电压 → 无光谱段

    target = float(d0.u[len(d0.u) // 2])
    summary = chip.summary(voltage=target)
    entry = [e for e in summary["devices"] if e is not None][0]
    assert entry["spectra"] is not None
    assert entry["spectra"]["matched_voltage"] == pytest.approx(
        float(d0.u[entry["spectra"]["matched_index"]])
    )

    # Device 级 / Chip 级 / site 过滤
    pt = d0.spectrum_at(target)
    assert pt is not None
    assert pt.requested_voltage == pytest.approx(target)
    assert pt.values.shape == (655,)
    assert pt.matched_voltage == pytest.approx(float(d0.u[pt.matched_index]))

    pts = chip.spectra_at(target)
    assert len(pts) >= 1
    site = d0.name.removeprefix("S").split("@")[0]  # 如 "1A"
    pts_filtered = chip.spectra_at(target, site=site)
    assert len(pts_filtered) == 1
    assert pts_filtered[0].device_name == d0.name


# ---------- 真实样例：旧格式 ----------

def test_old_format_parse_file(old_format_file):
    chip, warning = lp.parse_file(old_format_file)
    assert warning is None  # 旧格式光谱内嵌，无 warning
    devices = [d for d in chip.devices if d is not None]
    assert len(devices) >= 1
    d = devices[0]
    assert "@" in d.name
    assert d.spectra.ndim == 2
    assert d.wavelength.shape[0] == d.spectra.shape[1]
    assert d.spectra.shape[0] == d.u.shape[0]


def test_parse_dir_real(new_format_dir, old_format_dir):
    result = lp.parse_dir(new_format_dir)
    assert len(result.chips) >= 1
    assert isinstance(result.warnings, list)
    assert isinstance(result.errors, list)

    result = lp.parse_dir(old_format_dir)
    assert len(result.chips) >= 1
