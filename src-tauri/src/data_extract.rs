use calamine::{open_workbook, DataType, Reader, Xlsx};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};
use serde::Serialize;

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
        }
    }
}

fn is_empty_data(device_data: &DeviceData) -> bool {
    let mut result = true;
    for u_val in device_data.u.iter() {
        if *u_val != 0.0 {
            result = false;
            break;
        }
    }
    result
}

#[tauri::command]
pub fn open_data_file(path: String) -> String {
    let mut workbook: Xlsx<_> = open_workbook(path).unwrap();
    let sheets = workbook.worksheets();
    let u_length = sheets[0].1.height() - 2;
    let mut spc_length = 1;
    for sheet in sheets[1..].iter() {
        if sheet.1.height() > 0 {
            spc_length = sheet.1.height() - 1;
            break;
        }
    }

    let is_vis = sheets[0].1.width() == 320;

    let data_rows = (&sheets[0].1).rows().skip(2);
    let column_indecies_vis: [usize; 8] = [0, 40, 80, 120, 160, 200, 240, 280];
    let column_indecies_ir: [usize; 8] = [0, 11, 22, 33, 44, 55, 66, 77];

    let mut devices: Vec<DeviceData> = sheets[1..]
        .iter()
        .map(|(name, _)| DeviceData::new(&name, is_vis, u_length, spc_length))
        .collect();

    data_rows.into_iter().enumerate().for_each(|(index, row)| {
        if is_vis {
            column_indecies_vis
                .par_iter()
                .zip(devices.par_iter_mut())
                .for_each(|(col, device)| {
                    let lumi = row[*col + 3].as_f64().unwrap();
                    let eqe = if lumi > 0.0 {
                        row[*col + 4].as_f64().unwrap()
                    } else {
                        0.0
                    };
                    device.u[index] = row[*col].as_f64().unwrap();
                    device.j[index] = row[*col + 2].as_f64().unwrap();
                    device.luminance[index] = lumi;
                    device.eqe[index] = eqe;
                });
        } else {
            column_indecies_ir
                .par_iter()
                .zip(devices.par_iter_mut())
                .for_each(|(col, device)| {
                    let radi = row[*col + 6].as_f64().unwrap();
                    let eqe = if radi > 0.0 {
                        row[*col + 5].as_f64().unwrap()
                    } else {
                        0.0
                    };
                    device.u[index] = row[*col].as_f64().unwrap();
                    device.j[index] = row[*col + 7].as_f64().unwrap();
                    device.luminance[index] = radi;
                    device.eqe[index] = eqe;
                });
        }
    });

    let mut devices_filtered: Vec<Option<DeviceData>> = devices
        .into_par_iter()
        .map(|device| {
            if is_empty_data(&device) {
                None
            } else {
                Some(device)
            }
        })
        .collect();

    devices_filtered
        .par_iter_mut()
        .zip(sheets[1..].par_iter())
        .for_each(|(element, sheet)| {
            if let Some(device) = element {
                let spectra_rows = sheet.1.rows().skip(1);

                spectra_rows
                    .into_iter()
                    .enumerate()
                    .for_each(|(index, data)| {
                        device.wavelength[index] = data[0].as_f64().unwrap();
                        device
                            .spectra
                            .par_iter_mut()
                            .zip(data.par_iter().skip(1))
                            .for_each(|(spc, d)| {
                                spc[index] = d.as_f64().unwrap();
                            });
                    })
            }
        });

    let devices_json = serde_json::to_string(&devices_filtered).unwrap();

    devices_json
}
