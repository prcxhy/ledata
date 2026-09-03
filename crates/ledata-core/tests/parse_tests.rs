mod common;

use common::*;
use ledata_core::{
    chip_spectra_at, device_spectrum_at, matches_site, nearest_voltage_index, parse_dir,
    parse_file, Chip,
};

fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

// ---------- 新格式 ----------

#[test]
fn new_format_full_parse() {
    let dir = unique_temp_dir("new_full");
    let devices: Vec<NewDeviceSpec> = (0..2)
        .map(|d| NewDeviceSpec {
            site_name: format!("Site{}", d + 1),
            u: vec![0.0, 1.0, 2.0],
            j: vec![0.1, 0.2, 0.3].into_iter().map(|v| v * (d as f64 + 1.0)).collect(),
            lumi: vec![0.0, 10.0, 20.0].into_iter().map(|v| v * (d as f64 + 1.0)).collect(),
            eqe: vec![0.0, 5.0, 4.0].into_iter().map(|v| v * (d as f64 + 1.0)).collect(),
        })
        .collect();
    let wl = full_wavelength();
    let rows: Vec<Vec<f64>> = (0..2)
        .flat_map(|d| (0..3usize).map(move |i| full_intensities(d, i)))
        .collect();
    let path = write_new_format(&dir, "c1", &devices, Some((&wl, &rows)));

    let (chip, warning) = parse_file(&path).unwrap();
    assert!(warning.is_none());
    assert_eq!(chip.name, "c1");
    assert_eq!(chip.devices.len(), 8); // 固定 8 槽位

    let d0 = chip.devices[0].as_ref().unwrap();
    assert_eq!(d0.name, "S1@c1");
    assert!(d0.is_vis);
    assert_eq!(d0.u, vec![0.0, 1.0, 2.0]);
    assert_eq!(d0.luminance, vec![0.0, 10.0, 20.0]);
    assert_eq!(d0.max_j, 0.3);
    assert_eq!(d0.max_lumi, 20.0);
    assert_eq!(d0.max_eqe, 5.0);
    // 光谱窗口：全量第 115 点起 655 点
    assert_eq!(d0.wavelength.len(), VIS_SLICE_LEN);
    approx(d0.wavelength[0], wl[VIS_SLICE_START]);
    assert_eq!(d0.spectra.len(), 3);
    assert_eq!(d0.spectra[0].len(), VIS_SLICE_LEN);
    approx(d0.spectra[0][0], full_intensities(0, 0)[VIS_SLICE_START]);
    approx(d0.spectra[2][654], full_intensities(0, 2)[VIS_SLICE_START + 654]);

    // 槽位 2..7 为空
    assert!(chip.devices.iter().skip(2).all(|d| d.is_none()));

    let d1 = chip.devices[1].as_ref().unwrap();
    assert_eq!(d1.name, "S2@c1");
    assert_eq!(d1.max_lumi, 40.0);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn new_format_missing_spectra_warns() {
    let dir = unique_temp_dir("new_missing");
    let devices: Vec<NewDeviceSpec> = (0..2)
        .map(|d| NewDeviceSpec {
            site_name: format!("Site{}", d + 1),
            u: vec![0.0, 1.0],
            j: vec![0.1, 0.2],
            lumi: vec![0.0, 1.0],
            eqe: vec![0.0, 1.0],
        })
        .collect();
    let path = write_new_format(&dir, "c2", &devices, None);

    let (chip, warning) = parse_file(&path).unwrap();
    assert_eq!(warning.unwrap(), "20260101 000000,Data,c2 光谱文件缺失");
    let d0 = chip.devices[0].as_ref().unwrap();
    assert!(d0.wavelength.is_empty());
    assert!(d0.spectra.iter().all(|row| row.is_empty()));
    // 性能数据不受光谱缺失影响
    assert_eq!(d0.u, vec![0.0, 1.0]);

    // 无光谱时消费层返回 None
    assert!(device_spectrum_at(d0, 0, 1.0).is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn new_format_spectra_row_mismatch_warns() {
    let dir = unique_temp_dir("new_mismatch");
    let devices: Vec<NewDeviceSpec> = (0..2)
        .map(|d| NewDeviceSpec {
            site_name: format!("Site{}", d + 1),
            u: vec![0.0, 1.0],
            j: vec![0.1, 0.2],
            lumi: vec![0.0, 1.0],
            eqe: vec![0.0, 1.0],
        })
        .collect();
    let wl = full_wavelength();
    // 只给一半行数（2 行，应为 4 行）→ 不匹配
    let rows: Vec<Vec<f64>> = vec![full_intensities(0, 0), full_intensities(0, 1)];
    let path = write_new_format(&dir, "c3", &devices, Some((&wl, &rows)));

    let (chip, warning) = parse_file(&path).unwrap();
    assert_eq!(warning.unwrap(), "20260101 000000,Data,c3 光谱文件不匹配");
    let d0 = chip.devices[0].as_ref().unwrap();
    assert!(d0.wavelength.is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

// ---------- 旧格式 ----------

#[test]
fn old_format_full_parse_with_empty_device() {
    let dir = unique_temp_dir("old_full");
    let devices: Vec<OldDeviceSpec> = (0..8)
        .map(|d| OldDeviceSpec {
            u: vec![0.0, 1.0, 2.0],
            j: vec![0.1, 0.2, 0.3].into_iter().map(|v| v * (d as f64 + 1.0)).collect(),
            lumi: vec![0.0, 10.0, 20.0].into_iter().map(|v| v * (d as f64 + 1.0)).collect(),
            eqe: vec![0.0, 5.0, 4.0],
        })
        .collect();
    let path = write_old_format(&dir, "old1", &devices, Some(7));

    let (chip, warning) = parse_file(&path).unwrap();
    assert!(warning.is_none()); // 旧格式光谱内嵌，无 warning 通道
    assert_eq!(chip.name, "old1");
    assert_eq!(chip.devices.len(), 8);
    assert!(chip.devices[7].is_none()); // 全 0 器件被过滤

    let d0 = chip.devices[0].as_ref().unwrap();
    assert_eq!(d0.name, "A@old1");
    assert!(d0.is_vis);
    assert_eq!(d0.u, vec![0.0, 1.0, 2.0]);
    assert_eq!(d0.max_j, 0.3);
    // 光谱：spc=4 个波长点 × 3 个电压
    assert_eq!(d0.wavelength, vec![500.0, 501.0, 502.0, 503.0]);
    assert_eq!(d0.spectra.len(), 3);
    assert_eq!(d0.spectra[0].len(), 4);
    approx(d0.spectra[0][0], 0.0);
    approx(d0.spectra[2][3], (2 * 10 + 3) as f64 * 0.002);

    let d6 = chip.devices[6].as_ref().unwrap();
    assert_eq!(d6.name, "G@old1");
    assert_eq!(d6.max_j, 0.3 * 7.0);

    std::fs::remove_dir_all(&dir).ok();
}

// ---------- 格式回退 ----------

#[test]
fn fallback_from_old_to_new() {
    // 文件名含逗号（新格式命名）但主表宽度 10：旧格式宽度校验拒绝 → 新格式解析
    let dir = unique_temp_dir("fallback");
    let devices: Vec<NewDeviceSpec> = vec![NewDeviceSpec {
        site_name: "Site1".into(),
        u: vec![0.0, 1.0],
        j: vec![0.1, 0.2],
        lumi: vec![0.0, 1.0],
        eqe: vec![0.0, 1.0],
    }];
    let path = write_new_format(&dir, "c9", &devices, None);

    let (chip, warning) = parse_file(&path).unwrap();
    // 未提供光谱文件 → 忠实行为：出现"光谱文件缺失"warning
    assert_eq!(warning.unwrap(), "20260101 000000,Data,c9 光谱文件缺失");
    assert_eq!(chip.name, "c9");
    assert!(chip.devices[0].is_some());
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- parse_dir ----------

#[test]
fn parse_dir_mixed_formats_and_errors() {
    let dir = unique_temp_dir("dir_mixed");
    let devices: Vec<NewDeviceSpec> = vec![NewDeviceSpec {
        site_name: "Site1".into(),
        u: vec![0.0, 1.0],
        j: vec![0.1, 0.2],
        lumi: vec![0.0, 1.0],
        eqe: vec![0.0, 1.0],
    }];
    write_new_format(&dir, "cb", &devices, None);

    let old_devices: Vec<OldDeviceSpec> = (0..8)
        .map(|d| OldDeviceSpec {
            u: vec![0.0, 1.0],
            j: vec![0.1, 0.2],
            lumi: vec![0.0, 1.0],
            eqe: vec![0.0, 1.0],
        })
        .collect();
    write_old_format(&dir, "ca", &old_devices, None);

    // 非 xlsx 文件应被忽略
    std::fs::write(dir.join("notes.txt"), "ignore me").unwrap();

    let outcome = parse_dir(&dir).unwrap();
    assert_eq!(outcome.chips.len(), 2);
    assert_eq!(outcome.warnings, vec!["20260101 000000,Data,cb 光谱文件缺失".to_string()]);
    assert!(outcome.errors.is_empty());
    let names: Vec<&str> = outcome.chips.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"ca"));
    assert!(names.contains(&"cb"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn parse_dir_empty_dir_errors() {
    let dir = unique_temp_dir("dir_empty");
    let err = parse_dir(&dir).unwrap_err();
    assert_eq!(err.to_string(), "该目录为空");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn parse_dir_only_non_xlsx_errors_as_empty() {
    // 忠实行为：目录内没有任何 .xlsx 时即报"该目录为空"
    let dir = unique_temp_dir("dir_nosupp");
    std::fs::write(dir.join("a.txt"), "x").unwrap();
    let err = parse_dir(&dir).unwrap_err();
    assert_eq!(err.to_string(), "该目录为空");
    std::fs::remove_dir_all(&dir).ok();
}

// ---------- 消费层 ----------

fn hand_device(name: &str, u: Vec<f64>, spectra: Vec<Vec<f64>>) -> ledata_core::DeviceData {
    ledata_core::DeviceData {
        name: name.into(),
        is_vis: true,
        wavelength: vec![300.0, 400.0, 500.0],
        u: u.clone(),
        j: vec![0.0; u.len()],
        luminance: vec![0.0; u.len()],
        eqe: vec![0.0; u.len()],
        spectra,
        max_j: 0.0,
        max_lumi: 0.0,
        max_eqe: 0.0,
        valid_eqe: 0.0,
        leak_j: 0.0,
    }
}

#[test]
fn nearest_voltage_index_matches() {
    assert_eq!(nearest_voltage_index(&[0.0, 1.0, 2.0, 3.0], 2.1), 2);
    assert_eq!(nearest_voltage_index(&[0.0, 1.0, 2.0, 3.0], -5.0), 0);
    assert_eq!(nearest_voltage_index(&[0.0, 1.0, 2.0, 3.0], 9.0), 3);
    // 并列取先出现者
    assert_eq!(nearest_voltage_index(&[1.0, 2.0, 3.0], 2.5), 1);
    assert_eq!(nearest_voltage_index(&[], 1.0), 0);
}

#[test]
fn device_and_chip_spectra_at() {
    let chip = Chip {
        name: "c".into(),
        devices: vec![
            Some(hand_device("S1A@c", vec![0.0, 3.0, 3.5], vec![vec![1.0; 3], vec![2.0; 3], vec![3.0; 3]])),
            None,
            Some(hand_device("S2A@c", vec![0.0, 2.0], vec![vec![4.0; 3], vec![5.0; 3]])),
        ],
    };

    // Device 级
    let pt = device_spectrum_at(chip.devices[0].as_ref().unwrap(), 0, 3.4).unwrap();
    assert_eq!(pt.matched_index, 2);
    approx(pt.matched_voltage, 3.5);
    assert_eq!(pt.values, vec![3.0; 3]);
    assert_eq!(pt.slot, 0);

    // Chip 级：各自匹配（目标 1.9：器件0 u=[0,3,3.5]→index1(3.0)，器件2 u=[0,2]→index1(2.0)）
    let pts = chip_spectra_at(&chip, 1.9, None);
    assert_eq!(pts.len(), 2);
    assert_eq!(pts[0].matched_index, 1);
    assert_eq!(pts[0].values, vec![2.0; 3]);
    assert_eq!(pts[1].matched_index, 1);
    assert_eq!(pts[1].values, vec![5.0; 3]);

    // site 过滤（宽松匹配）
    let pts = chip_spectra_at(&chip, 1.9, Some("2A"));
    assert_eq!(pts.len(), 1);
    assert_eq!(pts[0].device_name, "S2A@c");
    let pts = chip_spectra_at(&chip, 1.9, Some("s2a@c"));
    assert_eq!(pts.len(), 1);
    assert!(matches_site("S2A@chip", "2a"));

    // 空槽位跳过；spectra 缺失返回 None
    assert!(device_spectrum_at(chip.devices[1].as_ref().unwrap_or(&ledata_core::DeviceData {
        name: "x".into(), is_vis: true, u: vec![], j: vec![], luminance: vec![], eqe: vec![],
        wavelength: vec![], spectra: vec![], max_j: 0.0, max_lumi: 0.0, max_eqe: 0.0,
        valid_eqe: 0.0, leak_j: 0.0,
    }), 1, 1.0).is_none());
}

#[test]
fn chip_summary_shape() {
    let chip = Chip {
        name: "c".into(),
        devices: vec![
            Some(hand_device("S1A@c", vec![0.5, 3.5], vec![vec![1.0; 3], vec![2.0; 3]])),
            None,
        ],
    };
    let s = chip.summary(None);
    assert_eq!(s.name, "c");
    assert_eq!(s.devices.len(), 2);
    assert!(s.devices[1].is_none());
    let d = s.devices[0].as_ref().unwrap();
    assert_eq!(d.slot, 0);
    assert_eq!(d.name, "S1A@c");
    assert_eq!(d.n_points, 2);
    approx(d.u_min, 0.5);
    approx(d.u_max, 3.5);
    assert!(d.spectra.is_none());

    let s = chip.summary(Some(3.0));
    let d = s.devices[0].as_ref().unwrap();
    let sp = d.spectra.as_ref().unwrap();
    approx(sp.requested_voltage, 3.0);
    approx(sp.matched_voltage, 3.5);
    assert_eq!(sp.matched_index, 1);
}

// ---------- GUI JSON 契约（字段名快照） ----------

#[test]
fn json_contract_field_names() {
    let dir = unique_temp_dir("contract");
    let devices: Vec<NewDeviceSpec> = vec![NewDeviceSpec {
        site_name: "Site1".into(),
        u: vec![0.0, 1.0],
        j: vec![0.1, 0.2],
        lumi: vec![0.0, 1.0],
        eqe: vec![0.0, 1.0],
    }];
    let path = write_new_format(&dir, "cc", &devices, None);
    let (chip, _) = parse_file(&path).unwrap();
    let json = serde_json::to_string(&chip).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let d = &v["devices"][0];
    for key in ["name", "is_vis", "u", "j", "luminance", "eqe", "wavelength", "spectra", "max_j", "max_lumi", "max_eqe", "valid_eqe", "leak_j"] {
        assert!(d.get(key).is_some(), "missing key: {key}");
    }
    assert!(v.get("name").is_some());
    std::fs::remove_dir_all(&dir).ok();
}
