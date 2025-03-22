<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { computed, Ref, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DataViewer from './components/DataViewer.vue';
import Chip from "./components/Chip.vue";
import { Chip as ChipData, DeviceData } from "./scripts/DeviceData";
import IconOpen from "./assets/folder-open.svg?component"
import IconExcel from "./assets/excel.svg?component"

const workingPath = ref("");

const dataIndecies: Ref<number[][]> = ref([]);

const CHIPS: Ref<(ChipData)[]> = ref([new ChipData('', [])]);

const dataShowing = computed(() => {
  let dataArray = dataIndecies.value.map((array, index) => {
    return array.map(i => {
      return CHIPS.value[index].devices[i]!
    })
  }).flat();
  return dataArray
});

const dataStatus = computed(() => {
  return CHIPS.value.map(chip => {
    return chip.devices.map(d => {
      if(d != null) {
        return true;
      } else {
        return false
      }
    })
  });
});

watch(dataStatus, newStatus => {
  dataIndecies.value = [];
  if(newStatus.length == 1) {
    let indecies: number[] = [];
    newStatus[0].forEach((exsist, index) => {
      if(exsist) {
        indecies.push(index);
      }
    })
    dataIndecies.value.push(indecies);
  } else if(newStatus.length > 1) {
    for(var i = 0; i < newStatus.length; i ++) {
      dataIndecies.value.push([]);
    }
  }
});

const filterMode = ['亮度/辐照度', '最大EQE', '有效EQE', '电流密度', '漏电流'];

const filterModeIndex = ref(0);

const filterMax = ref(0);
const filterMin = ref(0);

function updateMapping(newIndex: number) {
  let array: number[];

  let getArray = (callback: (device: DeviceData) => number) => {
    return CHIPS.value.map(chip => {
      let subArray: number[] = [];
      chip.devices.forEach(device => {
        if(device) {
          subArray.push(callback(device));
        }
      });
      return subArray;
    }).flat();
  }

  if(newIndex == 0) {
    array = getArray(device => device.max_lumi);
  }
  if(newIndex == 1) {
    array = getArray(device => device.max_eqe);
  }
  if(newIndex == 2) {
    array = getArray(device => device.valid_eqe);
  }
  if(newIndex == 3) {
    array = getArray(device => device.max_j);
  }
  if(newIndex == 4) {
    array = getArray(device => Math.log10(device.leak_j));
  }

  filterMax.value = Math.max(...array!);
  filterMin.value = Math.min(...array!);
}

watch(filterModeIndex, newIndex => {
  updateMapping(newIndex);
})

async function openDataFile() {
  let filePath = await open({
    multiple: false,
    filters: [{
      name: '器件数据',
      extensions: ['xlsx']
    }]
  });

  if (filePath) {
    workingPath.value = filePath;
    invoke("open_one_file", {path: filePath})
    .then(data => {
      let chip: ChipData = JSON.parse(data as string);
      CHIPS.value = [chip];
      updateMapping(filterModeIndex.value)
    });
  }
}

async function openDataPath() {
  let path = await open({
    directory: true,
    multiple: false
  });
  
  if (path) {
    workingPath.value = path;
    invoke("open_path", {path: path})
    .then(data => {
      let chips: ChipData[] = JSON.parse(data as string);
      CHIPS.value = chips;
      updateMapping(filterModeIndex.value)
    });
  }
}
</script>

<template>
  <nav>
    <button @click="openDataFile"><IconExcel/>打开文件</button>
    <button @click="openDataPath"><IconOpen/>打开文件夹</button>
    <p>{{ workingPath }}</p>
  </nav>
  <div id="content" :style="{ gridTemplateColumns: dataShowing.length > 0? '1fr auto': '1fr 5mm' }">
    <div id="selector">
      <div id="performance-mapping">
        <h1>{{ `器件性能Mapping | ${filterMode[filterModeIndex]}` }}</h1>
        <div id="filter-slot">
          <p v-for="(modeName, index) in filterMode" @click="filterModeIndex = index" :class="[
            'filter-option',
            index == filterModeIndex? 'option-selected': ''
            ]">
            {{ modeName }}
          </p>
        </div>
      </div>
      <div id="chips-container" :style="{ gridTemplateColumns: `repeat(${Math.min(CHIPS.length, 4)}, 4cm)` }">
        <Chip v-for="(chip, index) in CHIPS"
        :name="chip.name"
        :devices="chip.devices"
        :status="dataStatus[index]"
        :filterMode="filterModeIndex"
        :filterMax="filterMax"
        :filterMin="filterMin"
        v-model="dataIndecies[index]"/>
      </div>
    </div>
    <DataViewer :data="dataShowing"/>
  </div>
</template>