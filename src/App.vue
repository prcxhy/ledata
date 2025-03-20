<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { computed, Ref, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DataViewer from './components/DataViewer.vue';
import Chip from "./components/Chip.vue";
import { DeviceData } from "./scripts/DeviceData";
import IconOpen from "./assets/folder-open.svg?component"

const workingPath = ref("");

const dataIndex: Ref<number[]> = ref([]);

const DEVICES: Ref<(DeviceData | null)[]> = ref([null, null, null, null, null, null, null, null]);

const dataShowing = computed(() => {
  return dataIndex.value.map(index => DEVICES.value[index])
});

const dataStatus = computed(() => {
  return DEVICES.value.map(data => {
    if(data != null) {
      return true;
    } else {
      return false
    }
  });
});

watch(dataStatus, newStatus => {
  dataIndex.value = [];
  newStatus.forEach((exsist, index) => {
    if(exsist) {
      dataIndex.value.push(index);
    }
  })
});

async function openDataFile() {
  let filePath = await open({
    multiple: false,
    filters: [{
      name: '器件数据',
      extensions: ['xlsx', 'xls']
    }]
  });

  if (filePath) {
    workingPath.value = filePath;
    invoke("open_data_file", {path: filePath}).then(data => {
      let devices: DeviceData[] = JSON.parse(data as string);
      DEVICES.value = devices;
    });
  }
}
</script>

<template>
  <nav>
    <button @click="openDataFile"><IconOpen/>打开文件</button>
    <p>{{ workingPath }}</p>
  </nav>
  <div id="content" :style="{ gridTemplateColumns: dataShowing.length > 0? '1fr auto': '1fr 5mm' }">
    <div id="selector">
      <Chip :status="dataStatus" v-model="dataIndex"></Chip>
    </div>
    <DataViewer :data="dataShowing"/>
  </div>
</template>