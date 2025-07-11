<script setup lang="ts">
import * as echarts from 'echarts';
import { nextTick, ref, useTemplateRef, watch } from 'vue';
import { DeviceData, drawChart, drawSpectra } from '../scripts/DeviceData';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import IconCopy from '../assets/clipboard.svg?component';
import IconExport from '../assets/down-picture.svg?component';

const prop = defineProps<{
    data: (DeviceData | null)[]
}>();

var chart1: echarts.ECharts;
var chart2: echarts.ECharts;
var chart3: echarts.ECharts;

let plot1 = useTemplateRef('luminance-j-v');
let plot2 = useTemplateRef('eqe-luminance');
let plot3 = useTemplateRef('spectra');

const uLength = ref(1);
const indexOfLongestU = ref(0);
const uIndex = ref(0);
const logXAxis = ref(false);

watch(() => prop.data, newData => {
    uLength.value = 1;
    indexOfLongestU.value = 0;
    uIndex.value = 0;
    newData.forEach((device, index) => {
        if(device!.u.length > uLength.value) {
            uLength.value = device!.u.length;
            indexOfLongestU.value = index;
        }
    })
    if(chart1) {
        chart1.dispose();
    }
    if(chart2) {
        chart2.dispose();
    }
    if(chart3) {
        chart3.dispose();
    }
    nextTick().then(() => {
        chart1 = echarts.init(plot1.value! as HTMLDivElement);
        chart2 = echarts.init(plot2.value! as HTMLDivElement);
        chart3 = echarts.init(plot3.value! as HTMLDivElement);
        drawChart(chart1, chart2, chart3, newData, uIndex.value, logXAxis.value);
        // let result = SIMU_RESULT.value[newIndex];
        // drawChart(chart, result, indexAtExtraAxis.value);
    })
}, { immediate: true });

watch(uIndex, newIndex => {
    if(chart3) {
        chart3.dispose();
    }
    nextTick().then(() => {
        chart3 = echarts.init(plot3.value! as HTMLDivElement);
        drawSpectra(chart3, prop.data, newIndex);
        // let result = SIMU_RESULT.value[newIndex];
        // drawChart(chart, result, indexAtExtraAxis.value);
    })
});

watch(logXAxis, newVal => {
    let xAxisOption = (chart2.getOption().xAxis as {[key: string]: any}[])[0];
    if(newVal) {
        xAxisOption.type = 'log';
        chart2.setOption({
            xAxis: xAxisOption
        })
    } else {
        xAxisOption.type = 'value';
        chart2.setOption({
            xAxis: xAxisOption
        })
    }
})

const emit = defineEmits(["message"]);

function copyPerformance() {
    if(prop.data.length > 0) {
        let row0 = 'U';
        let row1 = 'V';
        let row2 = '';
        for(var i = 0; i < prop.data.length; i ++) {
            row0 += prop.data[i]!.is_vis? '\tJ\tLuminance\tEQE': '\tJ\tRadiance\tEQE';
            row1 += prop.data[i]!.is_vis? '\tmA/cm²\tcd/m²\t%': '\tmA/cm²\tW/sr/m²\t%';
            row2 += `\t${prop.data[i]!.name}\t${prop.data[i]!.name}\t${prop.data[i]!.name}`;
        }
        let str = `${row0}\n${row1}\n${row2}\n`;
        prop.data[indexOfLongestU.value]!.u.forEach((uVal, index) => {
            let row = prop.data.map(d => {
                if(d && d.u.length > index) {
                    return [d.j[index], d.luminance[index], d.eqe[index]]
                } else {
                    return ['','','']
                }
            }).flat();
            row.splice(0, 0, uVal);
            let rowString = row.join('\t') + '\n';
            str += rowString;
        })
        writeText(str);
        emit("message", "已将性能数据复制到剪贴板，可在Origin直接粘贴", "ok");
    }
}

function copySpectra() {
    if(prop.data.length > 0) {
        let row0 = 'Wavelength';
        let row1 = 'nm';
        let row2 = '';
        for(var i = 0; i < prop.data.length; i ++) {
            if(prop.data[i]!.u.length <= uIndex.value) {
                continue;
            }
            row0 +='\tIntensity';
            row2 +=`\t${prop.data[i]!.name}`;
        }
        let str = `${row0}\n${row1}\n${row2}\n`;
        prop.data[0]!.wavelength.forEach((lambda, index) => {
            let row = prop.data.map(d => {
                if(d && d.u.length > uIndex.value) {
                    return [d.spectra[uIndex.value][index]]
                } else {
                    return []
                }
            }).flat();
            row.splice(0, 0, lambda);
            let rowString = row.join('\t') + '\n';
            str += rowString;
        })
        writeText(str);
        emit("message", "已将当前光谱数据复制到剪贴板，可在Origin直接粘贴", "ok");
    }
}

function saveImage() {
    let url1 = chart1.getDataURL({
        type: 'png',
        pixelRatio: 4,
        backgroundColor: '#fff'
    });
    let link1 = document.createElement('a');
    link1.href = url1;
    link1.download = prop.data[0]!.is_vis? 'J/Luminance-V.png': 'J/Radiance-V.png';
    link1.click();

    let url2 = chart2.getDataURL({
        type: 'png',
        pixelRatio: 4,
        backgroundColor: '#fff'
    });
    let link2 = document.createElement('a');
    link2.href = url2;
    link2.download = prop.data[0]!.is_vis? 'EQE-Luminance.png': 'EQE-Radiance.png';
    link2.click();

    let url3 = chart3.getDataURL({
        type: 'png',
        pixelRatio: 4,
        backgroundColor: '#fff'
    });
    let link3 = document.createElement('a');
    link3.href = url3;
    link3.download = 'Spectra.png';
    link3.click();
    emit("message", "图片已导出", "ok");
}
</script>

<template>
    <div id="data-viewer">
        <div id="plots-container">
            <div class="plot-card">
                <div ref="luminance-j-v" class="plot"></div>
            </div>
            <div class="plot-card">
                <div ref="eqe-luminance" class="plot"></div>
                <div id="x-axis-mode">
                    <p>横轴模式</p>
                    <div id="x-axis-mode-switch">
                        <p :class="['filter-option', !logXAxis? 'option-selected': '']" @click="logXAxis = false">线性</p>
                        <p :class="['filter-option', logXAxis? 'option-selected': ''] " @click="logXAxis = true">对数</p>
                    </div>
                </div>
            </div>
            <div class="plot-card">
                <div ref="spectra" class="plot"></div>
                <div id="votage-slide">
                    <input type="range" min="0" :max="uLength - 1" v-model.number="uIndex">
                    <p>{{ `${prop.data[indexOfLongestU]? prop.data[indexOfLongestU]!.u[uIndex].toFixed(1): 0.0} V` }}</p>
                </div>
            </div>
        </div>
        <div v-show="prop.data.length > 0" id="plots-tools">
            <button @click="copyPerformance"><IconCopy />复制性能数据</button>
            <button @click="copySpectra"><IconCopy />复制光谱数据</button>
            <button @click="saveImage"><IconExport />导出图片</button>
        </div>
    </div>
</template>