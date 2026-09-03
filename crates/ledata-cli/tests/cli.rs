//! ledata-cli 集成测试：退出码、错误路径、JSON/TSV 输出（合成 fixture）。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::Command;

fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("ledata_cli_{tag}_{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 最小新格式 fixture（结构与 ledata-core/tests/common 一致），含光谱文件
fn write_new_format_fixture(dir: &Path, chip: &str) -> PathBuf {
    let devices = 2usize;
    let rows = 3usize; // 每器件数据行数
    let wl_full = 800usize;

    let path = dir.join(format!("20260101 000000,Data,{chip}.xlsx"));
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    let headers = ["Site", "VOLT(V)", "CURR(mA)", "CD", "PDCURR(A)", "Luminance", "EQE(%)", "PeakWL"];
    for (c, h) in headers.iter().enumerate() {
        ws.write(0, c as u16, *h).unwrap();
    }
    let mut row: u32 = 1;
    for d in 0..devices {
        for i in 0..rows {
            ws.write(row, 0, &format!("Site{}", d + 1)).unwrap();
            let u = (i + 1) as f64;
            ws.write(row, 1, u).unwrap();
            ws.write(row, 2, u * 0.5).unwrap();
            ws.write(row, 3, u).unwrap();
            ws.write(row, 5, u * 10.0).unwrap();
            ws.write(row, 6, u).unwrap();
            row += 1;
        }
        if d + 1 < devices {
            row += 1; // 分隔空行
        }
    }
    wb.save(&path).unwrap();

    let spc = dir.join(format!("20260101 000000,VASpectrum,{chip}.csv"));
    let mut content = String::from("SiteNumber,Voltage(V),");
    content += &(0..wl_full).map(|i| (200.0 + i as f64 * 0.5).to_string()).collect::<Vec<_>>().join(",");
    content += "\n";
    for d in 0..devices {
        for i in 0..rows {
            content += &format!("Site{},", (i + 1) as f64);
            content += &(0..wl_full)
                .map(|w| ((d * 100 + i * 10 + w) as f64 * 0.001).to_string())
                .collect::<Vec<_>>()
                .join(",");
            content += "\n";
        }
    }
    fs::write(spc, content).unwrap();
    path
}

fn cmd() -> Command {
    Command::cargo_bin("ledata-cli").unwrap()
}

#[test]
fn nonexistent_path_exit_2() {
    cmd()
        .arg("Z:/definitely/not/here.xlsx")
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("路径不存在"));
}

#[test]
fn empty_dir_exit_1() {
    let dir = unique_temp_dir("empty");
    cmd()
        .arg(&dir)
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("该目录为空"));
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn summary_json_and_voltage_segment() {
    let dir = unique_temp_dir("summary");
    let path = write_new_format_fixture(&dir, "c1");

    // 默认摘要：无数组字段，无光谱段
    let out = cmd().arg(&path).assert().success().code(0).get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["voltage"], serde_json::Value::Null);
    let d0 = &v["chips"][0]["devices"][0];
    assert_eq!(d0["name"], "S1@c1");
    assert!(d0.get("u").is_none(), "摘要不应含原始数组");
    assert!(d0["spectra"].is_null());

    // 指定电压 → 光谱段
    let out = cmd()
        .arg(&path)
        .args(["--voltage", "2.9"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let d0 = &v["chips"][0]["devices"][0];
    let sp = &d0["spectra"];
    assert!(!sp.is_null());
    assert_eq!(sp["matched_index"], 2); // u=[1,2,3] → 2.9 最近 3
    assert_eq!(sp["matched_voltage"], 3.0);

    // --full：含原始数组
    let out = cmd()
        .arg(&path)
        .arg("--full")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let d0 = &v["chips"][0]["devices"][0];
    assert_eq!(d0["u"].as_array().unwrap().len(), 3);
    assert_eq!(d0["spectra"].as_array().unwrap().len(), 3);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn site_filter_and_csv_outputs() {
    let dir = unique_temp_dir("csv");
    let path = write_new_format_fixture(&dir, "c2");

    // site 过滤后只剩 1 个器件
    let out = cmd()
        .arg(&path)
        .args(["--site", "2"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(v["chips"][0]["devices"][0].is_null());
    assert!(!v["chips"][0]["devices"][1].is_null());

    // performance TSV：GUI 同口径表头
    let out = cmd()
        .arg(&path)
        .args(["--csv", "performance"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let expected = concat!(
        "U\tJ\tLuminance\tEQE\tJ\tLuminance\tEQE\n",
        "V\tmA/cm²\tcd/m²\t%\tmA/cm²\tcd/m²\t%\n",
        "\tS1@c2\tS1@c2\tS1@c2\tS2@c2\tS2@c2\tS2@c2\n"
    );
    assert!(text.starts_with(expected));

    // spectra TSV 需要 --voltage
    cmd()
        .arg(&path)
        .args(["--csv", "spectra"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicates::str::contains("--csv spectra 需要 --voltage"));

    let out = cmd()
        .arg(&path)
        .args(["--csv", "spectra", "--voltage", "2.0"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.starts_with("Wavelength\tIntensity\tIntensity\nnm\n\tS1@c2\tS2@c2"));
    // 波长列 = 全量 [115..770] 窗口 → 655 数据行 + 3 表头行
    assert_eq!(text.lines().count(), 3 + 655);

    // 未知 --csv 取值
    cmd()
        .arg(&path)
        .args(["--csv", "bogus"])
        .assert()
        .failure()
        .code(2);

    fs::remove_dir_all(&dir).ok();
}
