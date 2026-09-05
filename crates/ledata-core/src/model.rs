use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chip {
    pub name: String,
    pub devices: Vec<Option<DeviceData>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceData {
    pub name: String,
    pub is_vis: bool,
    pub u: Vec<f64>,
    pub j: Vec<f64>,
    pub luminance: Vec<f64>,
    pub eqe: Vec<f64>,
    pub wavelength: Vec<f64>,
    pub spectra: Vec<Vec<f64>>,
    pub max_j: f64,
    pub max_lumi: f64,
    pub max_eqe: f64,
    pub valid_eqe: f64,
    pub leak_j: f64,
}

impl DeviceData {
    pub(crate) fn new(name: &str, is_vis: bool, u_length: usize, spc_length: usize) -> DeviceData {
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
    pub(crate) fn remove_invalid_voltage(&mut self) {
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
    pub(crate) fn calc_summary_data(&mut self) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_device() -> DeviceData {
        DeviceData {
            name: "S1A@chip".into(),
            is_vis: true,
            u: vec![0.0, 1.0, 2.0, 3.0],
            j: vec![0.0, 0.5, 1.5, 2.5],
            luminance: vec![0.0, 1.0, 10.0, 20.0],
            eqe: vec![0.0, 2.0, 5.0, 4.0],
            wavelength: vec![],
            spectra: vec![vec![]; 4],
            max_j: 0.0,
            max_lumi: 0.0,
            max_eqe: 0.0,
            valid_eqe: 0.0,
            leak_j: 0.0,
        }
    }

    #[test]
    fn calc_summary_data_metrics() {
        let mut d = plain_device();
        d.calc_summary_data();
        assert_eq!(d.max_j, 2.5);
        assert_eq!(d.max_lumi, 20.0);
        assert_eq!(d.max_eqe, 5.0);
        // 有效EQE：亮度 >= 20*0.1=2.0 的位置从 index 2 起，EQE 剩余最大 5.0
        assert_eq!(d.valid_eqe, 5.0);
        // 漏电流：亮度>0 之前（index 0..1）J 均值
        assert_eq!(d.leak_j, 0.0);
    }

    #[test]
    fn remove_invalid_voltage_truncates() {
        let mut d = plain_device();
        d.u = vec![0.0, 1.0, 2.0, 0.5]; // 末尾电压骤降
        d.j = vec![0.0, 0.5, 1.5, 9.9];
        d.luminance = vec![0.0, 1.0, 10.0, 1.0];
        d.eqe = vec![0.0, 2.0, 5.0, 1.0];
        d.remove_invalid_voltage();
        assert_eq!(d.u, vec![0.0, 1.0, 2.0]);
        assert_eq!(d.j, vec![0.0, 0.5, 1.5]);
        assert_eq!(d.luminance, vec![0.0, 1.0, 10.0]);
        assert_eq!(d.eqe, vec![0.0, 2.0, 5.0]);
    }

    #[test]
    fn remove_invalid_voltage_tolerates_small_fluctuation() {
        let mut d = plain_device();
        // 波动 0.3 V：v.ceil() < prev.floor() 不成立，不应截断
        d.u = vec![0.0, 1.0, 0.7, 1.5];
        d.remove_invalid_voltage();
        assert_eq!(d.u.len(), 4);
    }

    #[test]
    fn serde_json_roundtrip() {
        let mut d = plain_device();
        d.calc_summary_data();
        let chip = Chip { name: "c1".into(), devices: vec![Some(d), None] };
        let json = serde_json::to_string(&chip).unwrap();
        let back: Chip = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "c1");
        assert!(back.devices[0].is_some());
        assert!(back.devices[1].is_none());
        assert_eq!(back.devices[0].as_ref().unwrap().max_lumi, 20.0);
    }
}
