//! 合成 fixture 生成器：按解析器对表格结构的假设构造最小新/旧格式文件。
//! 结构依据真实样例（openpyxl 检查）：
//! - 新格式：表头行 + 每器件 L 行数据（col0="SiteN"）+ 全空分隔行（末器件无）；
//!   光谱为同目录 "…,VASpectrum,{chip}.csv"：首行波长（skip(2) 后 [115..770] 窗口），
//!   其后每 (器件, 电压点) 一行（skip(2) 后 skip(115).take(655)）。
//! - 旧格式：主表 2 表头行 + L 数据行，宽 320(vis)/88(ir)，8 器件列块跨 40(vis)/11(ir)；
//!   光谱 sheets "SiteA".."SiteH"：1 表头行 + spc 行（col0 波长，col1..=L 各电压强度）。
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub struct NewDeviceSpec {
    pub site_name: String,       // 如 "Site1"（器件名 = "S" + site_name[4..]）
    pub u: Vec<f64>,
    pub j: Vec<f64>,
    pub lumi: Vec<f64>,
    pub eqe: Vec<f64>,
}

/// 生成新格式 xlsx；spectra 为 None 时不出光谱文件（触发"光谱文件缺失"）。
/// spectra_wavelength / spectra_rows 均为切片前的完整数组（各 800 点）。
pub fn write_new_format(
    dir: &Path,
    chip: &str,
    devices: &[NewDeviceSpec],
    spectra: Option<(&[f64], &[Vec<f64>])>,
) -> PathBuf {
    let stem = format!("20260101 000000,Data,{chip}");
    let path = dir.join(stem + ".xlsx");
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();

    let headers = ["Site", "VOLT(V)", "CURR(mA)", "CD", "PDCURR(A)", "Luminance", "EQE(%)", "PeakWL"];
    for (c, h) in headers.iter().enumerate() {
        ws.write(0, c as u16, *h).unwrap();
    }

    let mut row: u32 = 1;
    let l = devices[0].u.len() as u32;
    for (d, dev) in devices.iter().enumerate() {
        for i in 0..l {
            ws.write(row, 0, &dev.site_name).unwrap();
            ws.write(row, 1, dev.u[i as usize]).unwrap();
            ws.write(row, 2, dev.j[i as usize] * 0.5).unwrap();
            ws.write(row, 3, dev.j[i as usize]).unwrap();
            ws.write(row, 5, dev.lumi[i as usize]).unwrap();
            ws.write(row, 6, dev.eqe[i as usize]).unwrap();
            row += 1;
        }
        if d + 1 < devices.len() {
            row += 1; // 全空分隔行
        }
    }
    wb.save(&path).unwrap();

    if let Some((wl, rows)) = spectra {
        let spc_path = dir.join(format!("20260101 000000,VASpectrum,{chip}.csv"));
        let mut content = String::from("SiteNumber,Voltage(V),");
        content += &wl.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
        content += "\n";
        let mut idx = 0usize;
        for (d, dev) in devices.iter().enumerate() {
            for (i, _u) in dev.u.iter().enumerate() {
                if idx >= rows.len() {
                    break; // 故意构造行数不足（光谱文件不匹配）时提前结束
                }
                content += &format!("Site{},{},", d + 1, _u);
                content += &rows[idx].iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
                content += "\n";
                idx += 1;
            }
        }
        fs::write(spc_path, content).unwrap();
    }

    path
}

pub const FULL_WL: usize = 800;
pub const VIS_SLICE_START: usize = 115;
pub const VIS_SLICE_LEN: usize = 655; // [115..770]

pub fn full_wavelength() -> Vec<f64> {
    (0..FULL_WL).map(|i| 200.0 + i as f64 * 0.5).collect()
}

pub fn full_intensities(dev: usize, u_idx: usize) -> Vec<f64> {
    (0..FULL_WL).map(|i| ((dev * 100 + u_idx * 10 + i) as f64) * 0.001).collect()
}

pub struct OldDeviceSpec {
    pub u: Vec<f64>,
    pub j: Vec<f64>,
    pub lumi: Vec<f64>,
    pub eqe: Vec<f64>,
}

/// 生成旧格式 xlsx（vis）。empty_device 指定的器件全 0（解析后应为 None）。
pub fn write_old_format(dir: &Path, chip: &str, devices: &[OldDeviceSpec], empty_device: Option<usize>) -> PathBuf {
    let path = dir.join(format!("{chip}.xlsx"));
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();

    // 宽度必须 320：首行写满表头
    for c in 0..320u16 {
        ws.write(0, c, "h").unwrap();
    }
    ws.write(1, 0, "u").unwrap();

    let l = devices[0].u.len() as u32;
    for (d, dev) in devices.iter().enumerate() {
        let empty = empty_device == Some(d);
        let base = (d * 40) as u16;
        for i in 0..l {
            let row = 2 + i;
            ws.write(row, base, if empty { 0.0 } else { dev.u[i as usize] }).unwrap();
            ws.write(row, base + 2, if empty { 0.0 } else { dev.j[i as usize] }).unwrap();
            ws.write(row, base + 3, if empty { 0.0 } else { dev.lumi[i as usize] }).unwrap();
            ws.write(row, base + 4, if empty { 0.0 } else { dev.eqe[i as usize] }).unwrap();
        }
    }
    wb.save(&path).unwrap();

    // 光谱 sheets：SiteA..SiteH
    let spc = 4usize;
    for (d, _dev) in devices.iter().enumerate() {
        let name = format!("Site{}", char::from(b'A' + d as u8));
        let sheet = wb.add_worksheet();
        sheet.set_name(&name).unwrap();
        sheet.write(0, 0, "wl").unwrap();
        for w in 0..spc {
            let row = (w + 1) as u32;
            sheet.write(row, 0, 500.0 + w as f64).unwrap();
            for iv in 0..l as u16 {
                let empty = empty_device == Some(d);
                let v = if empty { 0.0 } else { ((d * 100 + iv as usize * 10 + w) as f64) * 0.002 };
                sheet.write(row, 1 + iv, v).unwrap();
            }
        }
    }
    wb.save(&path).unwrap();
    path
}

pub fn unique_temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ledata_core_{}_{}_{}", tag, std::process::id(), chrono_suffix()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn chrono_suffix() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
}
