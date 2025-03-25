<script setup lang="ts">
import { Ref, ref, watch } from 'vue';
import ITO from '../assets/ITO2.svg?component';
import { DeviceData } from '../scripts/DeviceData';

const prop = defineProps<{
    name: string,
    devices: (DeviceData | null)[],
    status: boolean[];
    filterMode: number;
    filterMax: number;
    filterMin: number;
}>();
const deviceIds = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
const selectIndecies = defineModel<number[]>()

function updateSelect(index: number) {
    let i = selectIndecies.value!.indexOf(index);
    if(i >= 0) {
        selectIndecies.value!.splice(i, 1);
    } else {
        selectIndecies.value!.push(index);
    }
    selectIndecies.value = selectIndecies.value!.sort((a, b) => a - b);
}

function selectAll() {
    let indecies: number[] = [];
    prop.status.forEach((exsist, index) => {
        if(exsist) {
            indecies!.push(index);
        }
    });
    if(selectIndecies.value!.length == indecies.length) {
        selectIndecies.value = [];
    } else {
        selectIndecies.value = indecies;
    }
}

function getMappingColor(mode: number, device: DeviceData) {
    let r = 255;
    if(mode == 0) {
        r = Math.round((device.max_lumi - prop.filterMin) / (prop.filterMax - prop.filterMin) * 223);
    }
    if(mode == 1) {
        // let a = (device.max_eqe - prop.filterMin) / (prop.filterMax - prop.filterMin);
        // let b = (Math.log10(a * 9.9 + 0.1) + 1) / 2;
        // r = Math.round(b * 223);
        r = Math.round((device.max_eqe - prop.filterMin) / (prop.filterMax - prop.filterMin) * 223);
    }
    if(mode == 2) {
        // let a = (device.valid_eqe - prop.filterMin) / (prop.filterMax - prop.filterMin);
        // let b = (Math.log10(a * 9.9 + 0.1) + 1) / 2;
        // r = Math.round(b * 223);
        r = Math.round((device.valid_eqe - prop.filterMin) / (prop.filterMax - prop.filterMin) * 223);
    }
    if(mode == 3) {
        r = Math.round((device.max_j - prop.filterMin) / (prop.filterMax - prop.filterMin) * 223);
    }
    if(mode == 4) {
        // let a = (device.leak_j - prop.filterMin) / (prop.filterMax - prop.filterMin);
        // let b = (Math.log10(a * 9.999 + 0.001) + 3) / 4;
        // r = Math.round(b * 223);
        // r = Math.round((Math.log10(device.leak_j) - prop.filterMin) / (prop.filterMax - prop.filterMin) * 223);
        r = Math.round((device.leak_j - prop.filterMin) / (prop.filterMax - prop.filterMin) * 223);
    }
    if(Number.isNaN(r)) {
        return `rgb(255, 0, 255)`
    }
    return `rgb(${r + 32}, 0, 0)`
}

const mappingColor: Ref<string[], string[]> = ref([]);
watch(() => { return { mode: prop.filterMode, devices: prop.devices } }, newInput => {
    mappingColor.value = newInput.devices.map(devide => {
        if(devide) {
            return getMappingColor(newInput.mode, devide);
        } else {
            return 'rgb(128, 128, 128)';
        }
    })
}, { immediate: true })
</script>

<template>
    <div class="substrate" @click="selectAll">
        <div v-for="(id, index) in deviceIds" :class="['device', `Site-${id}`,
            (prop.status[index] && selectIndecies!.includes(index))? 'device-active': '',
            prop.status[index]? '': 'device-empty'
        ]"
        :style="{ backgroundColor: mappingColor[index] }"
        @click.stop="() => {
            if(prop.status[index]) {
                updateSelect(index)
            }
        }">
            <p class="device-label">{{ prop.status[index]? id: "空" }}</p>
        </div>
        <ITO />
        <p class="chip-name">{{ prop.name }}</p>
    </div>
</template>