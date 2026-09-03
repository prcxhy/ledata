//! ledata-py：ledata-core 的 PyO3 薄绑定层（类型转换 + 错误映射），无第二套解析逻辑。
//! 约定：numpy 数组每次属性访问都从 Rust 拷贝一份；全量导出走 to_json/to_dict 显式动作。

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::PyTypeInfo;

use ledata_core as core;

create_exception!(ledata_py, LeDataError, PyException, "LEData 基础异常");
create_exception!(ledata_py, UnsupportedFormatError, LeDataError, "表格的格式不受支持");
create_exception!(ledata_py, EmptyDirError, LeDataError, "目录为空");
create_exception!(ledata_py, NoSupportedDataError, LeDataError, "目录下没有支持格式的数据表格");
create_exception!(ledata_py, InvalidPathError, LeDataError, "路径无效");

fn map_err(err: core::CoreError) -> PyErr {
    let msg = err.to_string();
    match err {
        core::CoreError::UnsupportedFormat { .. } => UnsupportedFormatError::new_err(msg),
        core::CoreError::EmptyDir => EmptyDirError::new_err(msg),
        core::CoreError::NoSupportedData => NoSupportedDataError::new_err(msg),
    }
}

fn validate_file_path(path: &str) -> PyResult<()> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(InvalidPathError::new_err(format!("路径不存在: {path}")));
    }
    if p.is_dir() {
        return Err(InvalidPathError::new_err(format!("路径是目录而非文件: {path}")));
    }
    Ok(())
}

fn validate_dir_path(path: &str) -> PyResult<()> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(InvalidPathError::new_err(format!("路径不存在: {path}")));
    }
    if p.is_file() {
        return Err(InvalidPathError::new_err(format!("路径是文件而非目录: {path}")));
    }
    Ok(())
}

/// 单点光谱：某器件在匹配电压索引处的切片
#[pyclass(name = "SpectrumPoint", module = "ledata_py")]
struct SpectrumPointPy {
    inner: core::SpectrumPoint,
}

#[pymethods]
impl SpectrumPointPy {
    #[getter]
    fn device_name(&self) -> String {
        self.inner.device_name.clone()
    }
    #[getter]
    fn slot(&self) -> usize {
        self.inner.slot
    }
    #[getter]
    fn requested_voltage(&self) -> f64 {
        self.inner.requested_voltage
    }
    #[getter]
    fn matched_voltage(&self) -> f64 {
        self.inner.matched_voltage
    }
    #[getter]
    fn matched_index(&self) -> usize {
        self.inner.matched_index
    }
    #[getter]
    fn wavelength<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        numpy::PyArray1::from_vec(py, self.inner.wavelength.clone())
    }
    #[getter]
    fn values<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        numpy::PyArray1::from_vec(py, self.inner.values.clone())
    }
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        point_to_dict(py, &self.inner)
    }
}

fn point_to_dict<'py>(py: Python<'py>, p: &core::SpectrumPoint) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("device_name", &p.device_name)?;
    dict.set_item("slot", p.slot)?;
    dict.set_item("requested_voltage", p.requested_voltage)?;
    dict.set_item("matched_voltage", p.matched_voltage)?;
    dict.set_item("matched_index", p.matched_index)?;
    dict.set_item("wavelength", p.wavelength.clone())?;
    dict.set_item("values", p.values.clone())?;
    Ok(dict)
}

/// 器件数据对象；数组属性每次访问都返回拷贝
#[pyclass(name = "DeviceData", module = "ledata_py")]
struct DeviceDataPy {
    inner: core::DeviceData,
    slot: usize,
}

#[pymethods]
impl DeviceDataPy {
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    #[getter]
    fn is_vis(&self) -> bool {
        self.inner.is_vis
    }
    #[getter]
    fn slot(&self) -> usize {
        self.slot
    }
    #[getter]
    fn u<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        numpy::PyArray1::from_vec(py, self.inner.u.clone())
    }
    #[getter]
    fn j<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        numpy::PyArray1::from_vec(py, self.inner.j.clone())
    }
    #[getter]
    fn luminance<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        numpy::PyArray1::from_vec(py, self.inner.luminance.clone())
    }
    #[getter]
    fn eqe<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        numpy::PyArray1::from_vec(py, self.inner.eqe.clone())
    }
    #[getter]
    fn wavelength<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray1<f64>> {
        numpy::PyArray1::from_vec(py, self.inner.wavelength.clone())
    }
    /// 2D 矩阵，形状 (n_u, n_wavelength)，行 = 电压索引
    #[getter]
    fn spectra<'py>(&self, py: Python<'py>) -> Bound<'py, numpy::PyArray2<f64>> {
        numpy::PyArray2::from_vec2(py, &self.inner.spectra).unwrap()
    }
    #[getter]
    fn max_j(&self) -> f64 {
        self.inner.max_j
    }
    #[getter]
    fn max_lumi(&self) -> f64 {
        self.inner.max_lumi
    }
    #[getter]
    fn max_eqe(&self) -> f64 {
        self.inner.max_eqe
    }
    #[getter]
    fn valid_eqe(&self) -> f64 {
        self.inner.valid_eqe
    }
    #[getter]
    fn leak_j(&self) -> f64 {
        self.inner.leak_j
    }
    /// 全量字典（字段与 GUI JSON 契约一致 + slot）
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        device_to_dict(py, &self.inner, self.slot)
    }
    /// 全量 JSON（字段与 GUI JSON 契约一致）
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyException::new_err(format!("序列化失败: {e}")))
    }
    /// Device 级消费：该器件在 voltage 处匹配的光谱（无光谱数据返回 None）
    fn spectrum_at(&self, voltage: f64) -> Option<SpectrumPointPy> {
        core::device_spectrum_at(&self.inner, self.slot, voltage).map(|inner| SpectrumPointPy { inner })
    }
}

fn device_to_dict<'py>(
    py: Python<'py>,
    d: &core::DeviceData,
    slot: usize,
) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("name", &d.name)?;
    dict.set_item("slot", slot)?;
    dict.set_item("is_vis", d.is_vis)?;
    dict.set_item("u", d.u.clone())?;
    dict.set_item("j", d.j.clone())?;
    dict.set_item("luminance", d.luminance.clone())?;
    dict.set_item("eqe", d.eqe.clone())?;
    dict.set_item("wavelength", d.wavelength.clone())?;
    dict.set_item("spectra", d.spectra.clone())?;
    dict.set_item("max_j", d.max_j)?;
    dict.set_item("max_lumi", d.max_lumi)?;
    dict.set_item("max_eqe", d.max_eqe)?;
    dict.set_item("valid_eqe", d.valid_eqe)?;
    dict.set_item("leak_j", d.leak_j)?;
    Ok(dict)
}

/// 器件玻片（chip）数据对象
#[pyclass(name = "Chip", module = "ledata_py")]
struct ChipPy {
    inner: core::Chip,
}

fn summary_to_dict<'py>(py: Python<'py>, s: &core::ChipSummary) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("name", &s.name)?;
    let devices: Vec<_> = s
        .devices
        .iter()
        .map(|d| {
            d.as_ref().map(|d| {
                let dd = PyDict::new(py);
                dd.set_item("slot", d.slot).ok();
                dd.set_item("name", &d.name).ok();
                dd.set_item("is_vis", d.is_vis).ok();
                dd.set_item("max_j", d.max_j).ok();
                dd.set_item("max_lumi", d.max_lumi).ok();
                dd.set_item("max_eqe", d.max_eqe).ok();
                dd.set_item("valid_eqe", d.valid_eqe).ok();
                dd.set_item("leak_j", d.leak_j).ok();
                dd.set_item("n_points", d.n_points).ok();
                dd.set_item("u_min", d.u_min).ok();
                dd.set_item("u_max", d.u_max).ok();
                match &d.spectra {
                    Some(sp) => {
                        let spd = PyDict::new(py);
                        spd.set_item("requested_voltage", sp.requested_voltage).ok();
                        spd.set_item("matched_voltage", sp.matched_voltage).ok();
                        spd.set_item("matched_index", sp.matched_index).ok();
                        dd.set_item("spectra", spd).ok();
                    }
                    None => {
                        dd.set_item("spectra", pyo3::types::PyNone::get(py)).ok();
                    }
                }
                dd
            })
        })
        .collect();
    dict.set_item("devices", devices)?;
    Ok(dict)
}

#[pymethods]
impl ChipPy {
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    /// 8 槽位，空位为 None（对应 GUI 玻片 A-H）
    #[getter]
    fn devices(&self) -> Vec<Option<DeviceDataPy>> {
        self.inner
            .devices
            .iter()
            .enumerate()
            .map(|(slot, d)| d.as_ref().map(|d| DeviceDataPy { inner: d.clone(), slot }))
            .collect()
    }
    /// Agent 摘要（无原始数组）；voltage 传入时附加光谱匹配段
    #[pyo3(signature = (voltage=None))]
    fn summary<'py>(&self, py: Python<'py>, voltage: Option<f64>) -> PyResult<Bound<'py, PyDict>> {
        summary_to_dict(py, &self.inner.summary(voltage))
    }
    /// Chip 级消费：全部有数据器件各自匹配（同一目标电压）；site 可选过滤
    #[pyo3(signature = (voltage, site=None))]
    fn spectra_at(&self, voltage: f64, site: Option<&str>) -> Vec<SpectrumPointPy> {
        core::chip_spectra_at(&self.inner, voltage, site)
            .into_iter()
            .map(|inner| SpectrumPointPy { inner })
            .collect()
    }
    /// 全量 JSON（字段与 GUI JSON 契约一致）
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyException::new_err(format!("序列化失败: {e}")))
    }
}

/// 目录批量解析结果
#[pyclass(name = "DirResult", module = "ledata_py")]
struct DirResultPy {
    chips: Vec<core::Chip>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[pymethods]
impl DirResultPy {
    #[getter]
    fn chips(&self) -> Vec<ChipPy> {
        self.chips.iter().map(|c| ChipPy { inner: c.clone() }).collect()
    }
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.warnings.clone()
    }
    #[getter]
    fn errors(&self) -> Vec<String> {
        self.errors.clone()
    }
}

/// 解析单个 xlsx（先旧格式后新格式自动回退）；返回 (Chip, warning)
#[pyfunction]
fn parse_file(py: Python<'_>, path: String) -> PyResult<(ChipPy, Option<String>)> {
    validate_file_path(&path)?;
    let (chip, warning) = py
        .detach(|| core::parse_file(std::path::Path::new(&path)))
        .map_err(map_err)?;
    Ok((ChipPy { inner: chip }, warning))
}

/// 解析目录下全部 xlsx（并行）；chips 为空时抛 NoSupportedDataError
#[pyfunction]
fn parse_dir(py: Python<'_>, path: String) -> PyResult<DirResultPy> {
    validate_dir_path(&path)?;
    let outcome = py.detach(|| core::parse_dir(std::path::Path::new(&path))).map_err(map_err)?;
    Ok(DirResultPy {
        chips: outcome.chips,
        warnings: outcome.warnings,
        errors: outcome.errors,
    })
}

#[pymodule]
fn ledata_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("LeDataError", LeDataError::type_object(m.py()))?;
    m.add("UnsupportedFormatError", UnsupportedFormatError::type_object(m.py()))?;
    m.add("EmptyDirError", EmptyDirError::type_object(m.py()))?;
    m.add("NoSupportedDataError", NoSupportedDataError::type_object(m.py()))?;
    m.add("InvalidPathError", InvalidPathError::type_object(m.py()))?;
    m.add_class::<ChipPy>()?;
    m.add_class::<DeviceDataPy>()?;
    m.add_class::<SpectrumPointPy>()?;
    m.add_class::<DirResultPy>()?;
    m.add_function(wrap_pyfunction!(parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(parse_dir, m)?)?;
    Ok(())
}
