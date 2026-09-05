//! GUI"复制性能/光谱数据"导出口径的 Rust 实现（Python/CLI 与 GUI 同一口径）。
//! 数字格式化按 JS `Number.prototype.toString()` 规则实现，保证与 TS 版本逐字符一致。

use crate::model::DeviceData;

/// JS 风格 f64 格式化：最短往返数字 + 指数规则（|x| ≥ 1e21 或 < 1e-6 用指数表示）
pub fn js_number_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    let neg = v.is_sign_negative();
    let a = v.abs();
    if a == 0.0 {
        return "0".into(); // JS: (-0).toString() === "0"
    }

    // Rust Display 恒为十进制最短表示，据此拆出数字串与量级 n（值 = 0.digits × 10^n）
    let s = a.to_string();
    let (int_part, frac_part) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s.as_str(), ""),
    };
    let int_digits = int_part.trim_start_matches('0');
    let (digits, n) = if !int_digits.is_empty() {
        (format!("{int_digits}{frac_part}"), int_digits.len() as i32)
    } else {
        let leading = frac_part.len() - frac_part.trim_start_matches('0').len();
        (frac_part[leading..].to_string(), -(leading as i32))
    };
    let digits_trimmed = digits.trim_end_matches('0');
    let (digits, digits_is_zero) = if digits_trimmed.is_empty() { ("0", true) } else { (digits_trimmed, false) };
    if digits_is_zero {
        return "0".into();
    }
    let k = digits.len() as i32;

    let mut out = String::new();
    if neg {
        out.push('-');
    }
    if n > 21 || n <= -6 {
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        let e = n - 1;
        if e >= 0 {
            out.push('+');
        }
        out.push_str(&e.to_string());
    } else if k <= n {
        out.push_str(digits);
        for _ in k..n {
            out.push('0');
        }
    } else if n > 0 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else {
        out.push_str("0.");
        for _ in 0..(-n) {
            out.push('0');
        }
        out.push_str(digits);
    }
    out
}

/// GUI"复制性能数据"同口径：devices 为有数据的器件列表（每器件 3 列）
pub fn performance_csv(devices: &[DeviceData]) -> String {
    let mut row0 = String::from("U");
    let mut row1 = String::from("V");
    let mut row2 = String::new();
    for d in devices {
        let kind = if d.is_vis { "Luminance" } else { "Radiance" };
        let lumi_unit = if d.is_vis { "cd/m²" } else { "W/sr/m²" };
        row0 += &format!("\tJ\t{kind}\tEQE");
        row1 += &format!("\tmA/cm²\t{lumi_unit}\t%");
        row2 += &format!("\t{0}\t{0}\t{0}", d.name);
    }
    let mut out = format!("{row0}\n{row1}\n{row2}\n");
    // TS 以"严格大于"遍历更新最长者 → 并列时取首个
    let max_len = devices.iter().map(|d| d.u.len()).max().unwrap_or(0);
    let Some(longest) = devices.iter().find(|d| d.u.len() == max_len) else {
        return out;
    };
    for (index, u_val) in longest.u.iter().enumerate() {
        let mut row = js_number_to_string(*u_val);
        for d in devices {
            if d.u.len() > index {
                row += &format!(
                    "\t{}\t{}\t{}",
                    js_number_to_string(d.j[index]),
                    js_number_to_string(d.luminance[index]),
                    js_number_to_string(d.eqe[index])
                );
            } else {
                row += "\t\t\t";
            }
        }
        out += &row;
        out += "\n";
    }
    out
}

/// GUI"复制光谱数据"同口径：u_index 处各器件光谱按波长对齐
pub fn spectra_csv(devices: &[DeviceData], u_index: usize) -> String {
    let mut row0 = String::from("Wavelength");
    let row1 = "nm".to_string();
    let mut row2 = String::new();
    for d in devices {
        if d.u.len() <= u_index {
            continue;
        }
        row0 += "\tIntensity";
        row2 += &format!("\t{}", d.name);
    }
    let mut out = format!("{row0}\n{row1}\n{row2}\n");
    let Some(first) = devices.first() else {
        return out;
    };
    for (index, lambda) in first.wavelength.iter().enumerate() {
        let mut row = js_number_to_string(*lambda);
        for d in devices {
            if d.u.len() > u_index {
                row += &format!("\t{}", js_number_to_string(d.spectra[u_index][index]));
            }
        }
        out += &row;
        out += "\n";
    }
    out
}

#[cfg(test)]
mod tests {
    use super::js_number_to_string as js;

    #[test]
    fn js_number_formatting() {
        assert_eq!(js(0.0), "0");
        assert_eq!(js(100.0), "100");
        assert_eq!(js(0.5), "0.5");
        assert_eq!(js(123.456), "123.456");
        assert_eq!(js(1e-7), "1e-7");
        assert_eq!(js(1e-6), "0.000001");
        assert_eq!(js(1e21), "1e+21");
        assert_eq!(js(1.23e22), "1.23e+22");
        assert_eq!(js(-2.5), "-2.5");
        assert_eq!(js(0.30000000000000004), "0.30000000000000004");
        assert_eq!(js(3.0), "3");
    }
}
