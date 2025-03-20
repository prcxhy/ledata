<script setup lang="ts">
import ITO from '../assets/ITO2.svg?component';

const prop = defineProps<{
    status: boolean[];
}>();
const devices = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
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
</script>

<template>
    <div class="substrate" @click="selectAll">
        <div v-for="(id, index) in devices" :class="['device',
            (prop.status[index] && selectIndecies!.includes(index))? 'device-active': '',
            prop.status[index]? '': 'device-empty'
        ]"
        :id="`Site-${id}`" @click.stop="() => {
            if(prop.status[index]) {
                updateSelect(index)
            }
        }">
            <p class="device-label">{{ prop.status[index]? id: "空" }}</p>
        </div>
        <ITO />
    </div>
</template>