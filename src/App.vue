<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, Ref, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import DataViewer from './components/DataViewer.vue';
import Chip from "./components/Chip.vue";
import { Chip as ChipData, DeviceData } from "./scripts/DeviceData";
import IconOpen from "./assets/folder-open.svg?component";
import IconExcel from "./assets/excel.svg?component";
import IconHelp from "./assets/help.svg?component";
import IconArrow from "./assets/down.svg?component";

const appWebview = getCurrentWebviewWindow();

const workingPath = ref("");

const dataIndecies: Ref<number[][]> = ref([]);

const CHIPS: Ref<(ChipData)[]> = ref([new ChipData('', [])]);

const messageText = ref('');
const messageType = ref('ok')

function showMessage(text: string, type: 'ok' | 'error' | 'warn') {
  messageText.value = text;
  messageType.value = type;
  setTimeout(() => messageText.value = '', 3000);
}

appWebview.listen<string>('fail-to-open', (event) => {
  showMessage(event.payload, 'error');
});

appWebview.listen<string>('no-match-spc', (event) => {
  showMessage(event.payload, 'warn');
});

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

const selectionBoxWidth = ref(0);
const selectionBoxLeft = ref(0);

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
  nextTick(() => {
    let optionSelected = (document.getElementById('filter-slot') as HTMLDivElement).children[newIndex];
    selectionBoxWidth.value = (optionSelected as HTMLParagraphElement).clientWidth;
    selectionBoxLeft.value = (optionSelected as HTMLParagraphElement).offsetLeft;
  })
}, {immediate: true});

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
        let turple = (data as string[]);
        let chip: ChipData = JSON.parse(turple[0]);
        let warnMsg = turple[1];
        if (warnMsg) {
          showMessage(warnMsg, 'warn');
        }

        CHIPS.value = [chip];
        dataExclude.value = [Array.from({length: chip.devices.length}, () => false)];
        updateMapping(MappingModeIndex.value)
      }).catch(msg => {
        showMessage(msg, 'error');
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
      }).catch(msg => {
        showMessage(msg, 'error');
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
  <Teleport to="body">
    <Transition>
      <p ref="message" v-if="messageText != ''" :class="['message', messageType]">
        <span v-html="messageText"></span>
      </p>
    </Transition>
  </Teleport>
  <nav>
    <button @click="openDataFile">
      <IconExcel />打开文件
    </button>
    <button @click="openDataPath">
      <IconOpen />打开文件夹
    </button>
    <p>{{ workingPath }}</p>
  </nav>
  <div id="content" :style="{ right: dataShowing.length > 0 ? '0px' : '-127mm' }">
    <div id="selector">
      <div id="performance-mapping">
        <h1>{{ `器件性能Mapping | ${filterMode[MappingModeIndex]}` }}</h1>
        <button @click="showMappingInfo = !showMappingInfo">
          <IconHelp v-if="!showMappingInfo" />
          <IconArrow v-if="showMappingInfo" />
        </button>
        <Transition>
          <p v-if="showMappingInfo" id="mapping-mode-info">
            {{ MappingModeInfo[MappingModeIndex] }}
          </p>
        </Transition>
        <div id="filter-slot">
          <p v-for="(modeName, index) in filterMode" @click="MappingModeIndex = index" :class="[
            'filter-option',
            index == MappingModeIndex ? 'option-selected' : ''
          ]">
            {{ modeName }}
          </p>
          <div ref="selection-box" id="selection-box"
            :style="{ left: selectionBoxLeft + 'px', width: selectionBoxWidth + 'px' }"></div>
          <label for="mapping-column">{{ "列数" }}</label>
          <input id="mapping-column" type="text" v-model.number="mappingColumn">
        </div>
      </div>
      <div id="chips-container"
        :style="{ gridTemplateColumns: `repeat(${Math.min(CHIPS.length, Math.max(1, mappingColumn))}, 4cm)` }">
        <Chip v-for="(chip, index) in CHIPS" :name="chip.name" :chipIndex="index" :devices="chip.devices"
          :status="dataStatus[index]" :exclude="dataExclude[index]" :filterMode="MappingModeIndex"
          :filterMax="filterMax" :filterMin="filterMin" @exclude-toggle="excludeToggle" @exclude-all="excludeAll"
          v-model="dataIndecies[index]" />
      </div>
    </div>
    <DataViewer :data="dataShowing" @message="showMessage" />
  </div>
</template>

<style>
nav > button {
  border-radius: 1mm;
  padding-left: 1.5mm;
}

.message {
  background-color: rgba(250, 250, 250, 0.7);
  position: relative;
  padding: 2mm 3mm 2mm 2mm;
  border-radius: 2mm;
  top: 2mm;
  font-size: 3.6mm;
  margin: 0px auto;
  justify-self: center;
  z-index: 2;
  filter: drop-shadow(0px 0px 8px rgba(0, 0, 0, 0.1));
  overflow: hidden;
  user-select: none;
  backdrop-filter: blur(4px);
}

.ok {
  border: 2px solid rgba(59, 209, 39, 0.2);
}

.ok::before {
  content: '●';
  color: rgba(59, 209, 39, 0.5);
  margin-right: 2mm;
}

.error {
  border: 2px solid rgba(255, 88, 88, 0.2);
}

.error::before {
  content: '×';
  font-weight: bold;
  color: rgb(255, 88, 88);
  margin-right: 2mm;
}

.warn {
  border: 2px solid rgba(255, 192, 88, 0.3);
}

.warn::before {
  content: '⚠';
  font-weight: bold;
  color: rgb(255, 192, 88);
  margin-right: 2mm;
}

#selection-box {
  position: absolute;
  background-color: rgb(250, 250, 250);
  border: 1px solid white;
  height: 8mm;
  border-radius: 1mm;
  transition: 0.15s;
}

.v-enter-active,
.v-leave-active {
  transition: all 0.3s ease;
}

.v-enter-from,
.v-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>