import { ECharts } from 'echarts';

const fontGridColor = '#000';

class Chip {
  name: string;
  devices: (DeviceData | null)[];
  constructor(name: string, devices: (DeviceData | null)[]) {
    this.name = name;
    this.devices = devices;
  }
}

class DeviceData {
  name: string;
  is_vis: boolean;
  u: number[];
  j: number[];
  luminance: number[];
  eqe: number[];
  wavelength: number[];
  spectra: number[][];
  max_j: number;
  max_lumi: number;
  max_eqe: number;
  valid_eqe: number;
  leak_j: number;
  constructor(
    name: string, is_vis: boolean, u: number[], j: number[], luminance: number[],
    eqe: number[], wavelength: number[], spectra: number[][], max_j: number,
    max_lumi: number, max_eqe: number, valid_eqe: number, leak_j: number,
  ) {
    this.name = name;
    this.is_vis = is_vis;
    this.u = u;
    this.j = j;
    this.luminance = luminance;
    this.eqe = eqe;
    this.wavelength = wavelength;
    this.spectra = spectra;
    this.max_j = max_j;
    this.max_lumi = max_lumi;
    this.max_eqe = max_eqe;
    this.valid_eqe = valid_eqe;
    this.leak_j = leak_j;
  }
}

function getRangeMin(data: number[], def: number) {
  let min = Math.min(...data);
  if(min > 0) {
    let exponent = Math.ceil(Math.log10(min));
    let order = Math.pow(10, exponent);
    return Math.max((Math.floor(min / order)) * order, def);
  } else {
    return def;
  }
}

function getRangeMax(data: number[], offset: number) {
  let max = Math.max(...data);
  let exponent = Math.floor(Math.log10(max));
  let order = Math.pow(10, exponent);
  return (Math.ceil(max / order) + offset) * order;
}

async function drawSpectra(chart: ECharts, devices: (DeviceData | null)[], uIndex: number) {
  if(devices.length > 0 && !devices.includes(null)) {
    let wMin = devices[0]!.wavelength[0];
    let wMax = devices[0]!.wavelength[devices[0]!.wavelength.length - 1];
    let spcMaxs: number[] = [];
    let series: {}[] = [];
  
    devices.forEach(device => {
      if(device!.spectra.length > uIndex) {
        spcMaxs.push(Math.max(...device!.spectra[uIndex]));
    
        let points3: number[][] = new Array();
        device!.spectra[uIndex].forEach((value, index) => {
          points3.push([device!.wavelength[index], value]);
        });
        series.push({
          name: device!.name, type: 'line', data: points3,
          symbol: 'none',
        })
      }
    })
  
    chart.setOption({
      animation: false,
      tooltip: {
        trigger: 'item', padding: [0, 4],
        formatter: (params: { [key: string]: any }) => {
          return `${(params.value[0] as number).toFixed(2)} nm<br/>${(params.value[1] as number).toFixed(3)}`;
        },
        textStyle: { fontSize: 12 }
      },
      legend: { top: 24, left: 72, width: 72, 
        textStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        }, 
      },
      dataZoom: [
        { type: 'inside', yAxisIndex: 0 },
        { type: 'inside', xAxisIndex: 0 }
      ],
      grid: {
        show: true,
        borderColor: fontGridColor,
        backgroundColor: '#fff',
        top: 24, bottom: 48, right: 24, left: 72
      },
      xAxis: {
        type: 'value', nameLocation: 'center', nameGap: 24,
        name: "Wavelength (nm)", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: wMin,
        max: wMax,
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toFixed(0);
          }
        },
        splitLine: { show: false },
      },
      yAxis: {
        type: 'value', nameLocation: 'center', nameGap: 48,
        name: "Intensity", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: 0, max: getRangeMax(spcMaxs, 0),
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toExponential();
          }
        },
        splitLine: { show: false },
      },
      series: series
    })
  }
}

async function drawChart(chart1: ECharts, chart2: ECharts, chart3: ECharts, devices: (DeviceData | null)[], uIndex: number) {
  if(devices.length > 0 && !devices.includes(null)) {
    let series1: {}[] = [];
    let series2: {}[] = [];
    let series3: {}[] = [];
    let isVis = devices[0]!.is_vis;
    let uMin = devices[0]!.u[0];
    let uMax = devices[0]!.u[devices[0]!.u.length - 1];
    let wMin = devices[0]!.wavelength[0];
    let wMax = devices[0]!.wavelength[devices[0]!.wavelength.length - 1];
    let jMins: number[] = [];
    let jMaxs: number[] = [];
    let lMaxs: number[] = [];
    let eMins: number[] = [];
    let eMaxs: number[] = [];
    let spcMaxs: number[] = [];

    devices.forEach(device => {
      jMins.push(Math.min(...device!.j));
      jMaxs.push(Math.max(...device!.j));
      lMaxs.push(Math.max(...device!.luminance));
      eMins.push(Math.min(...device!.eqe));
      eMaxs.push(Math.max(...device!.eqe));

      let points1_1: number[][] = new Array();
      let points1_2: number[][] = new Array();
      device!.u.forEach((value, index) => {
        points1_1.push([value, device!.j[index]]);
        points1_2.push([value, device!.luminance[index]]);
      });
      series1.push({
        name: `J-${device!.name}`, type: 'line', data: points1_1,
        yAxisIndex: 0,
        symbolSize: 6,
      }, {
        name: `${device!.is_vis? 'L': 'R'}-${device!.name}`, type: 'line', data: points1_2,
        yAxisIndex: 1,
        symbol: 'rect',
        symbolSize: 6,
      });

      let points2: number[][] = new Array();
      device!.luminance.forEach((value, index) => {
        points2.push([value, device!.eqe[index]]);
      });
      series2.push({
        name: device!.name, type: 'line', data: points2,
        symbolSize: 6,
      })

      if(device!.spectra.length > uIndex) {
        spcMaxs.push(Math.max(...device!.spectra[uIndex]));

        let points3: number[][] = new Array();
        device!.spectra[uIndex].forEach((value, index) => {
          points3.push([device!.wavelength[index], value]);
        });
        series3.push({
          name: device!.name, type: 'line', data: points3,
          symbol: 'none',
        })
      }
    })
    
    chart1.setOption({
      animation: false,
      tooltip: {
        trigger: 'item', padding: [0, 4],
        formatter: (params: { [key: string]: any }) => {
          return `${(params.value[0] as number).toFixed(2)} V<br/>${Math.round(params.value[1] as number)}`;
        },
        textStyle: { fontSize: 12 }
      },
      legend: { top: 24, left: 72, width: 192,
        textStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        }, 
      },
      dataZoom: [
        { type: 'inside', yAxisIndex: [0, 1] },
        { type: 'inside', xAxisIndex: 0 }
      ],
      grid: {
        show: true,
        borderColor: fontGridColor,
        backgroundColor: '#fff',
        top: 24, bottom: 48, right: 72, left: 72
      },
      xAxis: {
        type: 'value', nameLocation: 'center', nameGap: 24,
        name: "U (V)", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: uMin,
        max: uMax,
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toFixed(0);
          }
        },
        splitLine: { show: false },
      },
      yAxis: [{
        type: 'log', nameLocation: 'center', nameGap: 48,
        name: "J (mA/cm²)", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: getRangeMin(jMins, 1e-3), max: getRangeMax(jMaxs, 2),
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toExponential();
          }
        },
        splitLine: { show: false },
      }, {
        type: 'log', nameLocation: 'center', nameGap: 48,
        name: isVis? "Luminance (cd/cm²)": "Radiance (W/sr/m²)", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: isVis? 1: 1e-3, max: getRangeMax(lMaxs, 6),
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toExponential();
          }
        },
        splitLine: {
          show: false
        }
      }],
      series: series1,
    })

    chart2.setOption({
      animation: false,
      tooltip: {
        trigger: 'item', padding: [0, 4],
        formatter: (params: { [key: string]: any }) => {
          let radianceUnit = isVis? "cd/cm²": "W/sr/m²";
          return `${(params.value[0] as number).toFixed(2)} ${radianceUnit}<br/>${(params.value[1] as number).toFixed(3)} %`;
        },
        textStyle: { fontSize: 12 }
      },
      legend: { bottom: 48, right: 24, width: 288,
        textStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        }, 
      },
      dataZoom: [
        { type: 'inside', yAxisIndex: 0 },
        { type: 'inside', xAxisIndex: 0 }
      ],
      grid: {
        show: true,
        borderColor: fontGridColor,
        backgroundColor: '#fff',
        top: 24, bottom: 48, right: 24, left: 60
      },
      xAxis: {
        type: 'value', nameLocation: 'center', nameGap: 24,
        name: isVis? "Luminance (cd/cm²)": "Radiance (W/sr/m²)", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: isVis? 1: 1e-3,
        max: getRangeMax(lMaxs, 0),
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toExponential();
          }
        },
        splitLine: { show: false },
      },
      yAxis: {
        type: 'log', nameLocation: 'center', nameGap: 36,
        name: "EQE (%)", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: getRangeMin(eMins, 0.01), max: Math.ceil(Math.max(...eMaxs)),
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
        },
        splitLine: { show: false },
      },
      series: series2
    })

    chart3.setOption({
      animation: false,
      tooltip: {
        trigger: 'item', padding: [0, 4],
        formatter: (params: { [key: string]: any }) => {
          return `${(params.value[0] as number).toFixed(2)} nm<br/>${(params.value[1] as number).toFixed(3)}`;
        },
        textStyle: { fontSize: 12 }
      },
      legend: { top: 24, left: 72, width: 72,
        textStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        }, 
      },
      dataZoom: [
        { type: 'inside', yAxisIndex: 0 },
        { type: 'inside', xAxisIndex: 0 }
      ],
      grid: {
        show: true,
        borderColor: fontGridColor,
        backgroundColor: '#fff',
        top: 24, bottom: 48, right: 24, left: 72
      },
      xAxis: {
        type: 'value', nameLocation: 'center', nameGap: 24,
        name: "Wavelength (nm)", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: wMin,
        max: wMax,
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toFixed(0);
          }
        },
        splitLine: { show: false },
      },
      yAxis: {
        type: 'value', nameLocation: 'center', nameGap: 48,
        name: "Intensity", nameTextStyle: {
          color: fontGridColor, fontFamily: 'Times New Roman'
        },
        min: 0, max: getRangeMax(spcMaxs, 0),
        axisLabel: {
          color: fontGridColor, fontFamily: 'Times New Roman',
          formatter: (value: number) => {
            return value.toExponential();
          }
        },
        splitLine: { show: false },
      },
      series: series3
    })
  }
}

export { Chip, DeviceData, drawChart, drawSpectra }