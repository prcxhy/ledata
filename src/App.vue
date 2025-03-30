<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { computed, Ref, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DataViewer from './components/DataViewer.vue';
import Chip from "./components/Chip.vue";
import { Chip as ChipData, DeviceData } from "./scripts/DeviceData";
import IconOpen from "./assets/folder-open.svg?component";
import IconExcel from "./assets/excel.svg?component";
import IconHelp from "./assets/help.svg?component";
import IconArrow from "./assets/down.svg?component";

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

const dataExclude: Ref<boolean[][]> = ref([[]]);

const dataStatus = computed(() => {
  return CHIPS.value.map(chip => {
    return chip.devices.map(d => {
      if (d != null) {
        return true;
      } else {
        return false
      }
    })
  });
});

watch(dataStatus, newStatus => {
  dataIndecies.value = [];
  if (newStatus.length == 1) {
    let indecies: number[] = [];
    newStatus[0].forEach((exsist, index) => {
      if (exsist) {
        indecies.push(index);
      }
    })
    dataIndecies.value.push(indecies);
  } else if (newStatus.length > 1) {
    for (var i = 0; i < newStatus.length; i++) {
      dataIndecies.value.push([]);
    }
  }
});

const filterMode = ['亮度/辐照度', '最大EQE', '有效EQE', '电流密度', '漏电流'];

const MappingModeInfo = [
  '提取每个器件的"亮度/辐照度"数据记录中的最大值进行mapping',
  '提取每个器件的"EQE"数据记录中的最大值进行mapping',
  '在此定义"有效EQE": 在记录的EQE数据中, 排除对应亮度小于该器件最大亮度10%的点后, 剩余EQE数据中的最大值。根据此进行mapping',
  '提取每个器件的"电流密度"数据记录中的最大值进行mapping',
  "提取每个器件的\"漏电流\"的平均值进行mapping, 漏电流越小则颜色越暗。 但由于代码实现上较难判断开压, 故漏电流选取⚠️可能不准⚠️, 此mapping仅供参考"
];

const MappingModeIndex = ref(0);

const filterMax = ref(0);
const filterMin = ref(0);

function updateMapping(newIndex: number) {
  let array: number[];

  let getArray = (callback: (device: DeviceData) => number) => {
    return CHIPS.value.map((chip, ch) => {
      let subArray: number[] = [];
      chip.devices.forEach((device, dv) => {
        if (device && !dataExclude.value[ch][dv]) {
          subArray.push(callback(device));
        }
      });
      return subArray;
    }).flat();
  }

  if (newIndex == 0) {
    array = getArray(device => device.max_lumi);
  }
  if (newIndex == 1) {
    array = getArray(device => device.max_eqe);
  }
  if (newIndex == 2) {
    array = getArray(device => device.valid_eqe);
  }
  if (newIndex == 3) {
    array = getArray(device => device.max_j);
  }
  if (newIndex == 4) {
    // array = getArray(device => Math.log10(device.leak_j));
    array = getArray(device => device.leak_j);
  }

  filterMax.value = Math.max(...array!);
  filterMin.value = Math.min(...array!);
}

watch(MappingModeIndex, newIndex => {
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
    invoke("open_one_file", { path: filePath })
      .then(data => {
        let chip: ChipData = JSON.parse(data as string);
        CHIPS.value = [chip];
        dataExclude.value = [Array.from({length: chip.devices.length}, () => false)];
        updateMapping(MappingModeIndex.value)
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
    invoke("open_path", { path: path })
      .then(data => {
        let chips: ChipData[] = JSON.parse(data as string);
        CHIPS.value = chips;
        dataExclude.value = chips.map(chip => Array.from({length: chip.devices.length}, () => false));
        updateMapping(MappingModeIndex.value)
      });
  }
}

const mappingColumn = ref(4);

const showMappingInfo = ref(false);

function excludeToggle(position: number[]) {
  let ch = position[0];
  let dv = position[1];
  dataExclude.value[ch][dv] = !dataExclude.value[ch][dv];
  updateMapping(MappingModeIndex.value);
}

function excludeAll(chipIndex: number) {
  if(dataExclude.value[chipIndex].includes(true)) {
    dataExclude.value[chipIndex] = Array.from({length: dataExclude.value[chipIndex].length}, () => false);
  } else {
    dataExclude.value[chipIndex] = [...dataStatus.value[chipIndex]];
  }
  updateMapping(MappingModeIndex.value);
}
</script>

<template>
  <nav>
    <button @click="openDataFile">
      <IconExcel />打开文件
    </button>
    <button @click="openDataPath">
      <IconOpen />打开文件夹
    </button>
    <p>{{ workingPath }}</p>
  </nav>
  <div id="content" :style="{ gridTemplateColumns: dataShowing.length > 0 ? '1fr auto' : '1fr 5mm' }">
    <div id="selector">
      <div id="performance-mapping">
        <h1>{{ `器件性能Mapping | ${filterMode[MappingModeIndex]}` }}</h1>
        <button @click="showMappingInfo = !showMappingInfo">
          <IconHelp v-if="!showMappingInfo" />
          <IconArrow v-if="showMappingInfo" />
        </button>
        <p v-if="showMappingInfo" id="mapping-mode-info">
          <!-- {{ MappingModeInfo[MappingModeIndex] + "。😘当mapping颜色没有正常显示/自动刷新时, 手动切换一下mapping模式即可恢复正常😘" }} -->
          {{ MappingModeInfo[MappingModeIndex] }}
        </p>
        <div id="filter-slot">
          <p v-for="(modeName, index) in filterMode" @click="MappingModeIndex = index" :class="[
            'filter-option',
            index == MappingModeIndex ? 'option-selected' : ''
          ]">
            {{ modeName }}
          </p>
          <label for="mapping-column">{{ "列数" }}</label>
          <input id="mapping-column" type="text" v-model.number="mappingColumn">
        </div>
      </div>
      <div id="chips-container"
        :style="{ gridTemplateColumns: `repeat(${Math.min(CHIPS.length, Math.max(1, mappingColumn))}, 4cm)` }">
        <Chip v-for="(chip, index) in CHIPS"
        :name="chip.name"
        :chipIndex="index"
        :devices="chip.devices"
        :status="dataStatus[index]"
        :exclude="dataExclude[index]"
        :filterMode="MappingModeIndex"
        :filterMax="filterMax"
        :filterMin="filterMin"
        @exclude-toggle="excludeToggle"
        @exclude-all="excludeAll"
        v-model="dataIndecies[index]" />
      </div>
    </div>
    <DataViewer :data="dataShowing" />
  </div>
</template>