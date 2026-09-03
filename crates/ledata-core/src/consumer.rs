//! 消费层：光谱数据解析进 `DeviceData` 之后的返回/导出行为。
//! 解析始终全量（与 GUI 同一份逻辑），本模块只在消费时按电压参数匹配返回。

use serde::Serialize;

use crate::model::{Chip, DeviceData};

/// 在电压列中找最接近 target 的索引（并列取先出现者）；空数组返回 0
pub fn nearest_voltage_index(u: &[f64], target: f64) -> usize {
    let mut best: Option<(usize, f64)> = None;
    for (i, v) in u.iter().enumerate() {
        let dist = (v - target).abs();
        match best {
            Some((_, bd)) if !(dist < bd) => {}
            _ => best = Some((i, dist)),
        }
    }
    best.map(|(i, _)| i).unwrap_or(0)
}

/// 单点光谱（某器件在某电压索引处的光谱切片）
#[derive(Serialize, Clone)]
pub struct SpectrumPoint {
    pub device_name: String,
    pub slot: usize,
    pub requested_voltage: f64,
    pub matched_voltage: f64,
    pub matched_index: usize,
    pub wavelength: Vec<f64>,
    pub values: Vec<f64>,
}

/// Device 级：取该器件在 voltage 处匹配的光谱；无光谱数据（缺失/未加载成功）时返回 None
pub fn device_spectrum_at(device: &DeviceData, slot: usize, voltage: f64) -> Option<SpectrumPoint> {
    if device.spectra.is_empty() || device.wavelength.is_empty() {
        return None;
    }
    let index = nearest_voltage_index(&device.u, voltage);
    let values = device.spectra.get(index)?.clone();
    if values.is_empty() {
        return None;
    }
    Some(SpectrumPoint {
        device_name: device.name.clone(),
        slot,
        requested_voltage: voltage,
        matched_voltage: device.u[index],
        matched_index: index,
        wavelength: device.wavelength.clone(),
        values,
    })
}

/// site 宽松匹配：`"1A"` / `"S1A@chip"` / `"S1A"` 互相等价（大小写不敏感）
pub fn matches_site(device_name: &str, site: &str) -> bool {
    fn normalize(s: &str) -> String {
        let no_chip = s.split('@').next().unwrap_or(s);
        let upper = no_chip.to_ascii_uppercase();
        let no_s = upper.strip_prefix('S').unwrap_or(&upper);
        no_s.to_string()
    }
    normalize(device_name) == normalize(site)
}

/// Chip 级：chip 内全部有数据器件各自在各自电压列匹配（8 器件同一目标电压）；
/// site 过滤时仅返回该器件
pub fn chip_spectra_at(chip: &Chip, voltage: f64, site: Option<&str>) -> Vec<SpectrumPoint> {
    let mut points = Vec::new();
    for (slot, device) in chip.devices.iter().enumerate() {
        if let Some(device) = device {
            if let Some(site) = site {
                if !matches_site(&device.name, site) {
                    continue;
                }
            }
            if let Some(point) = device_spectrum_at(device, slot, voltage) {
                points.push(point);
            }
        }
    }
    points
}

/// 光谱段摘要（CLI --voltage / 显式请求时填充）
#[derive(Serialize, Clone)]
pub struct SpectraSummary {
    pub requested_voltage: f64,
    pub matched_voltage: f64,
    pub matched_index: usize,
}

#[derive(Serialize, Clone)]
pub struct DeviceSummary {
    pub slot: usize,
    pub name: String,
    pub is_vis: bool,
    pub max_j: f64,
    pub max_lumi: f64,
    pub max_eqe: f64,
    pub valid_eqe: f64,
    pub leak_j: f64,
    pub n_points: usize,
    pub u_min: f64,
    pub u_max: f64,
    pub spectra: Option<SpectraSummary>,
}

#[derive(Serialize, Clone)]
pub struct ChipSummary {
    pub name: String,
    pub devices: Vec<Option<DeviceSummary>>,
}

impl Chip {
    /// Agent 摘要：复用解析时 calc_summary_data 的五指标，无原始数组。
    /// voltage 传入时为每个器件附上光谱匹配段（无光谱数据的器件为 null）
    pub fn summary(&self, voltage: Option<f64>) -> ChipSummary {
        ChipSummary {
            name: self.name.clone(),
            devices: self
                .devices
                .iter()
                .enumerate()
                .map(|(slot, device)| {
                    device.as_ref().map(|device| {
                        let spectra = voltage.and_then(|v| {
                            device_spectrum_at(device, slot, v).map(|p| SpectraSummary {
                                requested_voltage: p.requested_voltage,
                                matched_voltage: p.matched_voltage,
                                matched_index: p.matched_index,
                            })
                        });
                        DeviceSummary {
                            slot,
                            name: device.name.clone(),
                            is_vis: device.is_vis,
                            max_j: device.max_j,
                            max_lumi: device.max_lumi,
                            max_eqe: device.max_eqe,
                            valid_eqe: device.valid_eqe,
                            leak_j: device.leak_j,
                            n_points: device.u.len(),
                            u_min: device.u.iter().cloned().fold(f64::INFINITY, f64::min),
                            u_max: device.u.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                            spectra,
                        }
                    })
                })
                .collect(),
        }
    }
}
