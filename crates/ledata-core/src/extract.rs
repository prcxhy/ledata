use std::{fs, io::Seek, path::{Path, PathBuf}};

use calamine::{Data, DataType, Range, Reader, Xlsx, open_workbook};
use rayon::{iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
}, slice::ParallelSlice};

use crate::error::CoreError;
use crate::model::{Chip, DeviceData};

/// 目录批量解析结果；warnings/errors 与 GUI 弹窗/事件的文案逐字一致
pub struct DirOutcome {
    pub chips: Vec<Chip>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// 对应 GUI `open_one_file`：单文件解析，先旧格式后新格式回退
pub fn parse_file(path: &Path) -> Result<(Chip, Option<String>), CoreError> {
    let path_buf = path.to_path_buf();
    let parent_path = path_buf.parent().unwrap().to_path_buf();
    let chip_name = path_buf.file_stem().unwrap().to_str().unwrap();
    let mut workbook: Xlsx<_> = open_workbook(&path_buf).unwrap();

    match extract_device_data_old(chip_name.to_string(), &mut workbook) {
        Ok(chip_data) => Ok((chip_data, None)),
        Err(_) => match extract_device_data(chip_name.to_string(), &mut workbook, parent_path) {
            Ok((chip_data, warning)) => Ok((chip_data, warning)),
            Err(msg) => Err(CoreError::UnsupportedFormat { file: msg }),
        },
    }
}

/// 对应 GUI `open_path`：目录批量解析（事件 emit 由宿主 GUI 层完成）
pub fn parse_dir(path: &Path) -> Result<DirOutcome, CoreError> {
    let path_buf = path.to_path_buf();
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
        return Err(CoreError::EmptyDir);
    }

    let mut chips = Vec::<Chip>::new();
    let mut warn_msgs = Vec::<String>::new();
    let mut error_msgs = Vec::<String>::new();

    let read_results = paths
        .par_iter()
        .map(|path| {
            let chip_name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let mut workbook: Xlsx<_> = open_workbook(path).unwrap();

            match extract_device_data_old(chip_name.to_string(), &mut workbook) {
                Ok(chip_data) => Ok((chip_data, None)),
                Err(_) => match extract_device_data(chip_name.to_string(), &mut workbook, path_buf.clone()) {
                    Ok((chip_data, warning)) => Ok((chip_data, warning)),
                    Err(msg) => Err(msg + " 表格的格式不受支持")
                }
            }
        })
        .collect::<Vec<Result<(Chip, Option<String>), String>>>();

    for result in read_results.into_iter() {
        match result {
            Ok((chip_data, warning)) => {
                chips.push(chip_data);
                if warning.is_some() {
                    warn_msgs.push(warning.unwrap());
                }
            },
            Err(msg) => error_msgs.push(msg)
        }
    };

    if chips.len() == 0 {
        return Err(CoreError::NoSupportedData);
    }

    Ok(DirOutcome { chips, warnings: warn_msgs, errors: error_msgs })
}

fn is_empty_data(device_data: &DeviceData) -> bool {
    return !device_data
        .u
        .iter()
        .zip(device_data.luminance.iter())
        .any(|(u_val, l_val)| *u_val > 0.0 && *l_val > 0.0);
}

fn extract_device_data<T>(file_name: String, workbook: &mut Xlsx<T>, path_buf: PathBuf) -> Result<(Chip, Option<String>), String>
where
    T: Seek,
    T: std::io::Read,
{
    let sheets = workbook.worksheets();

    if sheets[0].1.height() <= 1 {
        return Err(file_name);
    }

    let chip_name = file_name.split(',').collect::<Vec<&str>>()[2].to_owned();

    let mut spc_file_str = String::new();

    for entry in path_buf.read_dir().unwrap() {
        if let Ok(e) = entry {
            let file_name = e.path().file_stem().unwrap().to_owned().into_string().unwrap();
            if file_name.ends_with(&("VASpectrum,".to_owned() + &chip_name)) {
                spc_file_str = fs::read_to_string(e.path()).unwrap();
                break;
            }
        }
    }

    let mut warning: Option<String> = None;

    let is_vis = true;

    let (wavelength_str, spc_data_str) = if spc_file_str.is_empty() {
        warning = Some(file_name.to_owned() + " 光谱文件缺失");
        ("", "")
    } else {
        spc_file_str.split_once('\n').unwrap()
    };

    let wavelength = if wavelength_str == "" {
        Vec::new()
    } else {
        wavelength_str.trim().split(',').skip(2).map(|piece| {
            piece.parse::<f64>().unwrap_or(0.0)
        }).collect::<Vec<f64>>()
    };

    let mut number_of_devices: usize = 1;

    for i in 0..sheets[0].1.height() {
        if sheets[0].1.get_value((i as u32, 0)).unwrap() == &Data::Empty {
            number_of_devices = number_of_devices + 1;
        }
    }

    let u_length = (sheets[0].1.height() - number_of_devices)/number_of_devices;

    let spc_str_vec: Vec<&str> = if !wavelength.is_empty() {
        let vec: Vec<&str> = spc_data_str.trim().split_whitespace().collect();
        if vec.len() == number_of_devices * u_length {
            vec
        } else {
            warning = Some(file_name.to_owned() + " 光谱文件不匹配");
            vec![""; number_of_devices * u_length]
        }
    } else {
        vec![""; number_of_devices * u_length]
    };

    let mut device_data_vec: Vec<Range<Data>> = Vec::new();

    for i in 0..number_of_devices {
        let row =
        (&sheets[0].1).range(
            ((i*(u_length + 1) + 1) as u32, 0),
            (((i + 1)*(u_length + 1) - 1) as u32, sheets[0].1.width() as u32));
        device_data_vec.push(row);
    }

    let mut have_data = vec![false; 8];

    let devices = device_data_vec
        .into_par_iter()
        .zip(spc_str_vec.par_chunks(u_length))
        .map(|(device_data, spc_rows)| {
            let mut device = DeviceData::new("init", is_vis, u_length, 0);
            device.wavelength = if warning.is_none() {
                if is_vis {
                    wavelength.as_slice()[115..770].to_vec()
                } else {
                    wavelength.clone()
                }
            } else {
                Vec::new()
            };

            device_data.rows().into_iter().enumerate().for_each(|(index, row)| {
                if index == 0 {
                    device.name = "S".to_owned() + &row[0].as_string().unwrap().split_off(4) + "@" + &chip_name;
                }
                let lumi_or_radi = if is_vis {
                    row[5].as_f64().unwrap_or(0.0).max(0.0)
                } else {
                    row[18].as_f64().unwrap_or(0.0).max(0.0)
                };
                let eqe = if lumi_or_radi > 0.0 {
                    row[6].as_f64().unwrap_or(0.0).max(0.0)
                } else {
                    0.0
                };
                device.u[index] = row[1].as_f64().unwrap_or(0.0);
                device.j[index] = row[3].as_f64().unwrap_or(0.0).max(0.0);
                device.luminance[index] = lumi_or_radi;
                device.eqe[index] = eqe;

                if warning.is_none() {
                    let spc_iter = spc_rows[index].split(',').skip(2).map(|piece| {
                        piece.parse::<f64>().unwrap_or(0.0)
                    });
                    device.spectra[index] = if is_vis {
                        spc_iter.skip(115).take(655).collect::<Vec<f64>>()
                    } else {
                        spc_iter.collect::<Vec<f64>>()
                    };
                }
            });

            device
        }).collect::<Vec<DeviceData>>();

    for device in devices.iter() {
        let code = device.name[1..=1].parse::<usize>().unwrap();
        have_data[code - 1] = true;
    }

    let mut devices_iter = devices.into_iter();

    let devices_filtered = have_data.into_iter().map(|yes| {
        if yes {
            let mut device = devices_iter.next().unwrap();
            device.remove_invalid_voltage();
            device.calc_summary_data();
            Some(device)
        } else {
            None
        }
    }).collect::<Vec<Option<DeviceData>>>();

    Ok((Chip {
        name: chip_name,
        devices: devices_filtered,
    }, warning))
}

fn extract_device_data_old<T>(chip_name: String, workbook: &mut Xlsx<T>) -> Result<Chip, String>
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
