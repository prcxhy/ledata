//! ledata-cli：ledata-core 之上的 clap 薄壳，无独立解析路径。
//! 默认输出摘要 JSON（保护 Agent 上下文）；--full 才输出全量。

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use ledata_core::Chip;
use serde_json::{json, Value};

#[derive(Parser)]
#[command(
    name = "ledata-cli",
    version,
    about = "LED 器件测试数据提取（xlsx），与 LEData GUI 同一解析核心",
    after_help = "退出码: 0 成功 / 1 数据类失败 / 2 用法错误"
)]
struct Args {
    /// xlsx 数据文件或目录（自动识别，等价 GUI"打开文件/打开文件夹"）
    path: String,

    /// 指定电压：摘要中附加各器件在 V 处匹配的光谱段；--csv spectra 用其匹配索引
    #[arg(long)]
    voltage: Option<f64>,

    /// 器件过滤（Device 级粒度），如 "1A" 或 "S1A@chip"
    #[arg(long)]
    site: Option<String>,

    /// 输出全量数据（含原始数组与光谱矩阵，体积大；默认仅摘要）
    #[arg(long)]
    full: bool,

    /// 以 GUI 导出口径输出 TSV（Origin/Excel 可直接粘贴）: performance | spectra
    #[arg(long)]
    csv: Option<String>,

    /// JSON 缩进美化（默认紧凑单行）
    #[arg(long)]
    pretty: bool,
}

/// site 过滤：不匹配的器件槽位置 None（与 GUI 玻片槽位对齐）
fn filter_site(chip: &mut Chip, site: &str) {
    for device in chip.devices.iter_mut() {
        if let Some(d) = device {
            if !ledata_core::matches_site(&d.name, site) {
                *device = None;
            }
        }
    }
}

fn chip_to_value(chip: &Chip, full: bool, voltage: Option<f64>) -> Value {
    if full {
        serde_json::to_value(chip).expect("Chip 序列化")
    } else {
        serde_json::to_value(chip.summary(voltage)).expect("ChipSummary 序列化")
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    let path = Path::new(&args.path);

    if !path.exists() {
        eprintln!("路径不存在: {}", args.path);
        return ExitCode::from(2);
    }

    let is_dir = path.is_dir();

    // 解析（目录 → DirOutcome；文件 → 单 chip 包装）
    let mut chips: Vec<Chip> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    if is_dir {
        match ledata_core::parse_dir(path) {
            Ok(outcome) => {
                chips = outcome.chips;
                warnings = outcome.warnings;
                errors = outcome.errors;
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
    } else {
        match ledata_core::parse_file(path) {
            Ok((chip, warning)) => {
                if let Some(w) = warning {
                    warnings.push(w);
                }
                chips.push(chip);
            }
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        }
    }

    // site 过滤（先于 CSV/摘要，保证各输出模式一致）
    if let Some(site) = &args.site {
        for chip in chips.iter_mut() {
            filter_site(chip, site);
        }
    }

    // TSV 导出（GUI 同口径）
    if let Some(csv) = &args.csv {
        match csv.as_str() {
            "performance" => {
                let devices: Vec<ledata_core::DeviceData> =
                    chips.iter().flat_map(|c| c.devices.iter().flatten()).cloned().collect();
                print!("{}", ledata_core::performance_csv(&devices));
            }
            "spectra" => {
                let Some(voltage) = args.voltage else {
                    eprintln!("--csv spectra 需要 --voltage 指定匹配电压");
                    return ExitCode::from(2);
                };
                for (i, chip) in chips.iter().enumerate() {
                    let devices: Vec<ledata_core::DeviceData> =
                        chip.devices.iter().flatten().cloned().collect();
                    if devices.is_empty() {
                        continue;
                    }
                    // 该 chip 的光谱电压索引取首个有数据器件的匹配
                    let u_index = ledata_core::nearest_voltage_index(&devices[0].u, voltage);
                    if i > 0 {
                        println!();
                    }
                    print!("{}", ledata_core::spectra_csv(&devices, u_index));
                }
            }
            other => {
                eprintln!("未知的 --csv 取值: {other}（可选 performance | spectra）");
                return ExitCode::from(2);
            }
        }
        let _ = std::io::stdout().flush();
        return ExitCode::SUCCESS;
    }

    // JSON 输出（默认摘要 / --full 全量）
    let chips_json: Vec<Value> = chips.iter().map(|c| chip_to_value(c, args.full, args.voltage)).collect();
    let payload = json!({
        "voltage": args.voltage,
        "site": args.site,
        "warnings": warnings,
        "errors": errors,
        "chips": chips_json,
    });
    let text = if args.pretty {
        serde_json::to_string_pretty(&payload).expect("序列化")
    } else {
        serde_json::to_string(&payload).expect("序列化")
    };
    println!("{text}");
    ExitCode::SUCCESS
}
