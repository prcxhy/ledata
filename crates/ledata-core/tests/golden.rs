//! 三方对拍（Rust 侧）：与 TS 产出的期望 TSV 逐字符比对。
//! 需要 LEDATA_GOLDEN_DIR 指向 run_golden.py 准备的目录；未设置时静默跳过。

use ledata_core::Chip;

#[test]
fn golden_matches_ts_outputs() {
    let Ok(dir) = std::env::var("LEDATA_GOLDEN_DIR") else {
        eprintln!("LEDATA_GOLDEN_DIR 未设置，跳过三方对拍（由 tests/golden/run_golden.py 驱动）");
        return;
    };
    let dir = std::path::Path::new(&dir);
    let input = std::fs::read_to_string(dir.join("golden_input.json")).expect("golden_input.json");
    let chip: Chip = serde_json::from_str(&input).expect("解析 golden_input.json");
    let non_null: Vec<ledata_core::DeviceData> = chip.devices.iter().flatten().cloned().collect();
    assert!(!non_null.is_empty());

    let u_index: usize = std::fs::read_to_string(dir.join("u_index.txt"))
        .expect("u_index.txt")
        .trim()
        .parse()
        .expect("u_index");

    let perf = ledata_core::performance_csv(&non_null);
    let spc = ledata_core::spectra_csv(&non_null, u_index);

    let expected_perf = std::fs::read_to_string(dir.join("expected_performance.tsv")).expect("expected_performance.tsv");
    let expected_spc = std::fs::read_to_string(dir.join("expected_spectra.tsv")).expect("expected_spectra.tsv");

    assert_eq!(perf, expected_perf, "性能导出口径与 TS 不一致");
    assert_eq!(spc, expected_spc, "光谱导出口径与 TS 不一致");
}
