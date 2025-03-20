<script setup lang="ts">
import * as echarts from 'echarts';
import { nextTick, ref, useTemplateRef, watch } from 'vue';
import { DeviceData, drawChart, drawSpectra } from '../scripts/DeviceData';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import IconCopy from '../assets/clipboard.svg?component';
import IconExport from '../assets/export.svg?component';

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

const uIndex = ref(0);

watch(() => prop.data, newData => {
    uLength.value = newData[0]? newData[0].u.length: 1;
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
        drawChart(chart1, chart2, chart3, newData, uIndex.value);
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
})

function copyPerformance() {
    if(prop.data.length > 0) {
        let row0 = 'U';
        let row1 = 'V';
        let row2 = prop.data[0]!.name;
        for(var i = 0; i < prop.data.length; i ++) {
            row0 +='\tJ\tLuminance\tEQE';
            row1 +='\tmA/cm2\tcd/cm2\t%';
            row2 +=`\t${prop.data[i]!.name}\t${prop.data[i]!.name}\t${prop.data[i]!.name}`;
        }
        let str = `${row0}\n${row1}\n${row2}\n`;
        prop.data[0]!.u.forEach((uVal, index) => {
            let row = prop.data.map(d => {
                if(d) {
                    return [d.j[index], d.luminance[index], d.eqe[index]]
                } else {
                    return []
                }
            }).flat();
            row.splice(0, 0, uVal);
            let rowString = row.join('\t') + '\n';
            str += rowString;
        })
        writeText(str);
    }
}

function copySpectra() {
    if(prop.data.length > 0) {
        let row0 = 'Wavelength';
        let row1 = 'nm';
        let row2 = prop.data[0]!.name;
        for(var i = 0; i < prop.data.length; i ++) {
            row0 +='\tIntensity';
            row2 +=`\t${prop.data[i]!.name}`;
        }
        let str = `${row0}\n${row1}\n${row2}\n`;
        prop.data[0]!.wavelength.forEach((lambda, index) => {
            let row = prop.data.map(d => {
                if(d) {
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
    }
}

function saveImage() {
    let canvas1 = (plot1.value!.children[0].children[0] as HTMLCanvasElement);
    let url1 = canvas1.toDataURL('image/png');
    let link1 = document.createElement('a');
    link1.href = url1;
    link1.download = prop.data[0]!.is_vis? 'J/Luminance-V.png': 'J/Radiance-V.png';
    link1.click();

    let canvas2 = (plot2.value!.children[0].children[0] as HTMLCanvasElement);
    let url2 = canvas2.toDataURL('image/png');
    let link2 = document.createElement('a');
    link2.href = url2;
    link2.download = prop.data[0]!.is_vis? 'EQE-Luminance.png': 'EQE-Radiance.png';
    link2.click();

    let canvas3 = (plot3.value!.children[0].children[0] as HTMLCanvasElement);
    let url3 = canvas3.toDataURL('image/png');
    let link3 = document.createElement('a');
    link3.href = url3;
    link3.download = 'Spectra.png';
    link3.click();
}
</script>

<template>
    <div id="data-viewer">
        <div id="plots-container">
            <div ref="luminance-j-v" class="plot"></div>
            <div ref="eqe-luminance" class="plot"></div>
            <div class="plot">
                <div ref="spectra" class="plot"></div>
                <div id="votage-slide">
                    <input type="range" min="0" :max="uLength - 1" v-model.number="uIndex">
                    <p>{{ `${prop.data[0]? prop.data[0].u[uIndex].toFixed(1): 0.0} V` }}</p>
                </div>
            </div>
        </div>
        <div id="plots-tools">
            <button @click="copyPerformance"><IconCopy />复制性能数据</button>
            <button @click="copySpectra"><IconCopy />复制光谱数据</button>
            <button @click="saveImage"><IconExport />导出图片</button>
        </div>
    </div>
</template>