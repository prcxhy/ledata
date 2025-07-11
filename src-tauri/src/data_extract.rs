use std::{io::Seek, path::PathBuf};

use calamine::{open_workbook, DataType, Reader, Xlsx};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, EventTarget};

#[derive(Serialize)]
struct Chip {
    name: String,
    devices: Vec<Option<DeviceData>>,
}

#[derive(Serialize)]
struct DeviceData {
    name: String,
    is_vis: bool,
    u: Vec<f64>,
    j: Vec<f64>,
    luminance: Vec<f64>,
    eqe: Vec<f64>,
    wavelength: Vec<f64>,
    spectra: Vec<Vec<f64>>,
    max_j: f64,
    max_lumi: f64,
    max_eqe: f64,
    valid_eqe: f64,
    leak_j: f64,
}

impl DeviceData {
    fn new(name: &str, is_vis: bool, u_length: usize, spc_length: usize) -> DeviceData {
        DeviceData {
            name: name.to_string(),
            is_vis,
            u: vec![0.0; u_length],
            j: vec![0.0; u_length],
            luminance: vec![0.0; u_length],
            eqe: vec![0.0; u_length],
            wavelength: vec![0.0; spc_length],
            spectra: vec![vec![0.0; spc_length]; u_length],
            max_j: 0.0,
            max_lumi: 0.0,
            max_eqe: 0.0,
            valid_eqe: 0.0,
            leak_j: 0.0,
        }
    }
    fn remove_invalid_voltage(&mut self) {
        // 电压下降（击穿、烧坏）判断
        let voltage_index_until = self.u.iter().enumerate().position(|(i, v)| {
            if i == 0 {
                return false
            } else {
                // 电压波动应该不会被误判……吧
                return v < &self.u[i - 1] && v.ceil() < self.u[i - 1].floor()
            }
        });
        if let Some(until) = voltage_index_until {
            self.u.drain(until..);
            self.j.drain(until..);
            self.luminance.drain(until..);
            self.eqe.drain(until..);
            self.spectra.drain(until..);
        }
    }
    fn calc_summary_data(&mut self) {
        self.max_lumi = self
            .luminance
            .clone()
            .into_iter()
            .reduce(f64::max)
            .unwrap_or(0.);
        self.max_eqe = self.eqe.clone().into_iter().reduce(f64::max).unwrap_or(0.);
        self.max_j = self.j.clone().into_iter().reduce(f64::max).unwrap_or(0.);

        // 寻找有效EQE范围（不低于最高亮度10%）起始
        let valid_index_from = self
            .luminance
            .iter()
            .position(|l| *l >= self.max_lumi * 0.1)
            .unwrap_or(0);  // 找不到就设为数组头吧
        // 寻找漏电流截止位置（开压位置），找不到就设为数组尾吧
        let leak_index_until = self.luminance.iter().position(|l| *l > 0.0).unwrap_or(self.u.len());
        /*
            唉，真遇上奇怪数据找不准这些值，要是搞成异常处理麻烦得要死，
            加上数据主体又不是读不进来，这里不对劲也不能算作异常，
            就这样给个奇怪的值出来，到时候可视化的时候显示不对劲交给用户判断去吧
        */ 

        self.valid_eqe = self.eqe[valid_index_from..]
            .to_vec()
            .into_iter()
            .reduce(f64::max)
            .unwrap_or(0.);
        self.leak_j = self.j[..leak_index_until]
            .iter()
            .fold(0.0, |acc, j| acc + j)
            / (leak_index_until as f64);
    }
}

fn is_empty_data(device_data: &DeviceData) -> bool {
    return !device_data
        .u
        .iter()
        .zip(device_data.luminance.iter())
        .any(|(u_val, l_val)| *u_val > 0.0 && *l_val > 0.0);
}

fn extract_device_data<T>(chip_name: String, workbook: &mut Xlsx<T>) -> Result<Chip, String>
where
    T: Seek,
    T: std::io::Read,
{
    let sheets = workbook.worksheets();
    let u_length = sheets[0].1.height() - 2;
    let mut spc_length = 1;
    for sheet in sheets[1..].iter() {
        if sheet.1.height() > 0 {
            spc_length = sheet.1.height() - 1;
            break;
        }
    }

    if sheets[0].1.width() != 320 && sheets[0].1.width() != 88 {
        return Err(chip_name);
    }

    let is_vis = sheets[0].1.width() == 320;

    let data_rows = (&sheets[0].1).rows().skip(2);
    let column_indecies_vis: [usize; 8] = [0, 40, 80, 120, 160, 200, 240, 280];
    let column_indecies_ir: [usize; 8] = [0, 11, 22, 33, 44, 55, 66, 77];

    let mut devices: Vec<DeviceData> = sheets[1..]
        .iter()
        .map(|(name, _)| {
            let device_name = name.clone().split_off(4) + "@" + &chip_name;
            DeviceData::new(&device_name, is_vis, u_length, spc_length)
        })
        .collect();

    data_rows.into_iter().enumerate().for_each(|(index, row)| {
        if is_vis {
            column_indecies_vis
                .par_iter()
                .zip(devices.par_iter_mut())
                .for_each(|(col, device)| {
                    let lumi = row[*col + 3].as_f64().unwrap_or(0.0).max(0.0);
                    let eqe = if lumi > 0.0 {
                        row[*col + 4].as_f64().unwrap_or(0.0).max(0.0)
                    } else {
                        0.0
                    };
                    device.u[index] = row[*col].as_f64().unwrap_or(0.0);
                    device.j[index] = row[*col + 2].as_f64().unwrap_or(0.0).max(0.0);
                    device.luminance[index] = lumi;
                    device.eqe[index] = eqe;
                });
        } else {
            column_indecies_ir
                .par_iter()
                .zip(devices.par_iter_mut())
                .for_each(|(col, device)| {
                    let radi = row[*col + 6].as_f64().unwrap_or(0.0).max(0.0);
                    let eqe = if radi > 0.0 {
                        row[*col + 5].as_f64().unwrap_or(0.0).max(0.0)
                    } else {
                        0.0
                    };
                    device.u[index] = row[*col].as_f64().unwrap_or(0.0);
                    device.j[index] = row[*col + 7].as_f64().unwrap_or(0.0).max(0.0);
                    device.luminance[index] = radi;
                    device.eqe[index] = eqe;
                });
        }
    });

    let mut devices_filtered: Vec<Option<DeviceData>> = devices
        .into_par_iter()
        .map(|mut device| {
            if is_empty_data(&device) {
                None
            } else {
                device.remove_invalid_voltage();
                Some(device)
            }
        })
        .collect();

    devices_filtered
        .par_iter_mut()
        .zip(sheets[1..].par_iter())
        .for_each(|(element, sheet)| {
            if let Some(device) = element {
                device.calc_summary_data();

                let spectra_rows = sheet.1.rows().skip(1);

                spectra_rows
                    .into_iter()
                    .enumerate()
                    .for_each(|(index, data)| {
                        device.wavelength[index] = data[0].as_f64().unwrap_or(0.0);
                        device
                            .spectra
                            .par_iter_mut()
                            .zip(data.par_iter().skip(1))
                            .for_each(|(spc, d)| {
                                spc[index] = d.as_f64().unwrap_or(0.0);
                            });
                    })
            }
        });

    Ok(Chip {
        name: chip_name,
        devices: devices_filtered,
    })
}

#[tauri::command]
pub fn open_one_file(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(path);
    let chip_name = path_buf.file_stem().unwrap().to_str().unwrap();
    let mut workbook: Xlsx<_> = open_workbook(&path_buf).unwrap();

    match extract_device_data(chip_name.to_string(), &mut workbook) {
        Ok(chip_data) => Ok(serde_json::to_string(&chip_data).unwrap()),
        Err(msg) => Err(msg + " 表格的格式不受支持")
    }
}

#[tauri::command]
pub fn open_path(app: AppHandle,path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(path);
    let mut paths = Vec::<PathBuf>::new();
    for entry in path_buf.read_dir().unwrap() {
        if let Ok(e) = entry {
            let file_name = e.file_name().into_string().unwrap();
            if file_name.ends_with(".xlsx") {
                paths.push(path_buf.join(file_name));
            }
        }
    }

    if paths.len() == 0 {
        return Err("该目录为空".to_string());
    }

    let mut chips: Vec<Chip> = Vec::new();
    let mut error_msgs: Vec<String> = Vec::new();

    let read_results = paths
        .par_iter()
        .map(|path| {
            let chip_name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let mut workbook: Xlsx<_> = open_workbook(path).unwrap();

            extract_device_data(chip_name, &mut workbook)
        })
        .collect::<Vec<Result<Chip, String>>>();

    for result in read_results.into_iter() {
        match result {
            Ok(chip_data) => chips.push(chip_data),
            Err(msg) => error_msgs.push(msg)
        }
    };

    if chips.len() == 0 {
        return Err("该目录下没有找到符合支持格式的数据表格".to_string());
    } else if error_msgs.len() != 0 {
        let error_text = error_msgs.join("<br>");
        app.emit_to(EventTarget::any(), "fail-to-open", "以下文件的表格格式不受支持:<br>".to_string() + &error_text).unwrap();
    }

    Ok(serde_json::to_string(&chips).unwrap())
}
