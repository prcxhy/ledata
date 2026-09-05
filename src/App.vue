<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, Ref, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { resourceDir, join } from "@tauri-apps/api/path";
import { getVersion } from "@tauri-apps/api/app";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { openUrl } from "@tauri-apps/plugin-opener";
import DataViewer from './components/DataViewer.vue';
import Chip from "./components/Chip.vue";
import { Chip as ChipData, DeviceData } from "./scripts/DeviceData";
import IconOpen from "./assets/folder-open.svg?component";
import IconExcel from "./assets/excel.svg?component";
import IconHelp from "./assets/help.svg?component";
import IconArrow from "./assets/down.svg?component";
import IconAgent from "./assets/api.svg?component";
import IconAbout from "./assets/info.svg?component";
import IconUpdate from "./assets/update.svg?component";
import IconGithub from "./assets/github.svg?component";

const appWebview = getCurrentWebviewWindow();

const workingPath = ref("");

const dataIndecies: Ref<number[][]> = ref([]);

const CHIPS: Ref<(ChipData)[]> = ref([new ChipData('', [])]);

interface Toast {
  id: number;
  text: string;
  type: 'ok' | 'error' | 'warn';
}

const toasts = ref<Toast[]>([]);
let toastSeq = 0;

function showMessage(text: string, type: 'ok' | 'error' | 'warn') {
  const id = ++toastSeq;
  toasts.value.push({ id, text, type });
  setTimeout(() => {
    toasts.value = toasts.value.filter(t => t.id !== id);
  }, 3000);
}

async function copyAgentOnboarding() {
  try {
    const dir = (await resourceDir()).replace(/[\\/]+$/, "");
    const skillPath = await join(dir, "skill", "SKILL.md");
    const promptText =
      `我已安装 LEData 桌面应用（LED 器件测试数据工具），它自带面向 Agent 的数据接口技能。请按以下步骤完成接入：\n` +
      `1. 读取 "${skillPath}"；\n` +
      `2. 将文件中所有 <LEDATA_INSTALL_DIR> 占位符替换为 "${dir}"；\n` +
      `3. 将修改后的内容安装为你的技能（保存到你的技能目录，例如 ~/.agents/skills/ledata-python-interface/SKILL.md，或按你所用的技能安装机制处理）；\n` +
      `4. 安装完成后，我后续提到 LED 数据分析时请优先使用该技能中的接口。`;
    await writeText(promptText);
    showMessage("已复制提示词，请粘贴到你的 Agent 对话中完成接入", "ok");
  } catch (e) {
    showMessage(String(e), "error");
  }
}

// ---------- About / 版本更新检测 ----------

const RELEASE_API = "https://api.github.com/repos/prcxhy/ledata/releases/latest";
const RELEASES_PAGE = "https://github.com/prcxhy/ledata/releases/latest";
const CHECK_INTERVAL = 24 * 60 * 60 * 1000; // 当天首次启动测一次：24h 内的缓存直接复用

const appVersion = ref("");
const hasUpdate = ref(false);
const latestVersion = ref("");
const updateCheckFailed = ref(false);
const showAbout = ref(false);

function semverGt(a: string, b: string): boolean {
  const pa = a.split(".").map(n => parseInt(n, 10));
  const pb = b.split(".").map(n => parseInt(n, 10));
  for (var i = 0; i < 3; i ++) {
    let x = pa[i] || 0;
    let y = pb[i] || 0;
    if (x !== y) {
      return x > y;
    }
  }
  return false;
}

function parseTagVersion(tag: string): string | null {
  let m = tag.match(/^app-v(\d+\.\d+\.\d+)$/);
  return m? m[1]: null;
}

function applyLatest(latest: string | null) {
  if (latest) {
    latestVersion.value = latest;
    hasUpdate.value = semverGt(latest, appVersion.value);
    updateCheckFailed.value = false;
  } else {
    updateCheckFailed.value = true;
  }
}

async function checkForUpdate(force = false) {
  try {
    if (appVersion.value === "") {
      appVersion.value = await getVersion();
    }
    if (!force) {
      let cached = localStorage.getItem("ledata.update-check");
      if (cached) {
        let entry = JSON.parse(cached);
        if (Date.now() - entry.checkedAt < CHECK_INTERVAL) {
          applyLatest(entry.latest ?? null);
          return;
        }
      }
    }
    let resp = await fetch(RELEASE_API);
    let latest = parseTagVersion((await resp.json()).tag_name);
    // 检测失败也记录时间：当天不再重试
    localStorage.setItem("ledata.update-check", JSON.stringify({ checkedAt: Date.now(), latest }));
    applyLatest(latest);
  } catch {
    localStorage.setItem("ledata.update-check", JSON.stringify({ checkedAt: Date.now(), latest: null }));
    updateCheckFailed.value = true;
  }
}

// 当天第一次启动时检测一次（24h 内有缓存则跳过）
checkForUpdate();

function openAbout() {
  showAbout.value = true;
  // 从未成功检测过（如启动时断网）→ 打开时兜底测一次
  let cached = localStorage.getItem("ledata.update-check");
  if (!cached || !JSON.parse(cached).latest) {
    checkForUpdate(true);
  }
}

function openLink(url: string) {
  openUrl(url).catch(e => showMessage(String(e), "error"));
}

const aboutLinks = [
  { icon: IconGithub, label: "仓库地址", url: "https://github.com/prcxhy/ledata" },
  { icon: null, label: "问题反馈 (Issues)", url: "https://github.com/prcxhy/ledata/issues" },
  { icon: null, label: "GPL-3.0 License", url: "https://www.gnu.org/licenses/gpl-3.0.html" },
];

const aboutCredits = [
  { group: "框架", items: [
    { name: "Tauri", url: "https://github.com/tauri-apps/tauri" },
    { name: "Vue", url: "https://github.com/vuejs/core" },
    { name: "Vite", url: "https://github.com/vitejs/vite" },
    { name: "TypeScript", url: "https://github.com/microsoft/TypeScript" },
  ]},
  { group: "功能库", items: [
    { name: "ECharts", url: "https://github.com/apache/echarts" },
    { name: "calamine", url: "https://github.com/tafia/calamine" },
    { name: "rayon", url: "https://github.com/rayon-rs/rayon" },
    { name: "PyO3", url: "https://github.com/PyO3/pyo3" },
    { name: "maturin", url: "https://github.com/PyO3/maturin" },
    { name: "clap", url: "https://github.com/clap-rs/clap" },
    { name: "tauri 官方插件集", url: "https://github.com/tauri-apps/plugins-workspace" },
  ]},
];

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
    <div id="toasts">
      <TransitionGroup name="v">
        <p v-for="toast in toasts" :key="toast.id" :class="['message', toast.type]">
          <span v-html="toast.text"></span>
        </p>
      </TransitionGroup>
    </div>
  </Teleport>
  <Teleport to="body">
    <Transition>
      <div v-if="showAbout" id="about-overlay" @click.self="showAbout = false">
        <div id="about-dialog">
          <button id="about-close" @click="showAbout = false">✕</button>
          <h1>LEData <span>v{{ appVersion }}</span></h1>
          <p class="about-desc">LED 器件测试数据可视化浏览器，针对犀谱光电 XPQY-EQE 测试设备数据。</p>
          <div id="about-update">
            <p v-if="hasUpdate" class="update-available">
              <IconUpdate />新版本 {{ latestVersion }} 可用
              <button @click="openLink(RELEASES_PAGE)">前往下载更新</button>
            </p>
            <p v-else-if="updateCheckFailed" class="update-failed">未能检测更新</p>
            <p v-else class="update-latest">已是最新版本</p>
          </div>
          <div class="about-links">
            <p v-for="link in aboutLinks" :key="link.url" class="about-link" @click="openLink(link.url)">
              <component :is="link.icon" v-if="link.icon" />
              {{ link.label }}
            </p>
          </div>
          <h2>开源致谢</h2>
          <div v-for="group in aboutCredits" :key="group.group" class="about-credit-group">
            <p class="credit-group">{{ group.group }}</p>
            <p v-for="item in group.items" :key="item.url" class="about-link" @click="openLink(item.url)">
              {{ item.name }}
            </p>
          </div>
          <p class="credit-tail">以及所有间接依赖的开源项目</p>
        </div>
      </div>
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
    <button @click="copyAgentOnboarding">
      <IconAgent />Agent 接入
    </button>
    <button @click="openAbout">
      <IconAbout />关于
      <span v-if="hasUpdate" id="about-badge"></span>
    </button>
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

#toasts {
  position: fixed;
  top: 2mm;
  left: 0;
  right: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2mm;
  z-index: 2;
  pointer-events: none;
}

.message {
  background-color: rgba(250, 250, 250, 0.7);
  padding: 2mm 3mm 2mm 2mm;
  border-radius: 2mm;
  font-size: 3.6mm;
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

nav > button {
  position: relative;
}

#about-badge {
  position: absolute;
  top: -1mm;
  right: -1mm;
  width: 3mm;
  height: 3mm;
  border-radius: 50%;
  background-color: rgb(59, 209, 39);
  border: 1px solid white;
}

#about-overlay {
  position: fixed;
  inset: 0px;
  background-color: rgba(0, 0, 0, 0.3);
  display: grid;
  place-items: center;
  z-index: 3;
}

#about-dialog {
  background-color: rgb(250, 250, 250);
  border: 1px solid white;
  border-radius: 2mm;
  filter: drop-shadow(0px 0px 8px rgba(0, 0, 0, 0.2));
  padding: 6mm;
  width: 110mm;
  user-select: none;
}

#about-dialog h1 {
  font-size: 6mm;
  margin: 0px 0px 2mm 0px;
}

#about-dialog h1 span {
  color: rgb(128, 128, 128);
  font-size: 4mm;
  font-weight: normal;
}

#about-dialog h2 {
  font-size: 4.5mm;
  margin: 4mm 0px 1mm 0px;
}

#about-close {
  float: right;
  font-size: 5mm;
  width: 6mm;
  height: 6mm;
  justify-content: center;
  color: rgb(128, 128, 128);
}

.about-desc {
  color: rgb(96, 96, 96);
  margin: 0px 0px 3mm 0px;
}

#about-update {
  background-color: rgb(240, 240, 240);
  border-radius: 2mm;
  padding: 2mm 3mm;
  margin-bottom: 3mm;
}

#about-update > p {
  display: flex;
  align-items: center;
  gap: 1.5mm;
}

#about-update svg {
  width: 4.5mm;
  height: 4.5mm;
}

.update-available button {
  border-radius: 1mm;
  background-color: rgb(223, 223, 223);
  padding: 0px 2mm;
  height: 7mm;
}

.update-available {
  color: rgb(40, 120, 40);
  font-weight: bold;
}

.update-failed,
.update-latest {
  color: rgb(128, 128, 128);
}

.about-links {
  display: flex;
  gap: 4mm;
  margin-bottom: 2mm;
}

.about-links > p {
  display: flex;
  align-items: center;
  gap: 1mm;
}

.about-links svg {
  width: 4mm;
  height: 4mm;
}

.about-credit-group {
  display: flex;
  flex-wrap: wrap;
  gap: 1mm 3mm;
  margin-bottom: 1.5mm;
}

.credit-group {
  color: rgb(128, 128, 128);
  min-width: 12mm;
}

.about-link {
  color: rgb(64, 64, 64);
  cursor: pointer;
}

.about-link:hover {
  text-decoration: underline;
  color: black;
}

.credit-tail {
  color: rgb(160, 160, 160);
  font-size: 3.5mm;
}
</style>
