<template>
  <div class="data-explorer">
    <div class="panel-header">Data Explorer</div>
    <div class="panel-content">
      <div class="control-group">
        <label>Symbol</label>
        <input
          v-model="localSymbol"
          @change="updateSymbol"
          class="symbol-input"
          placeholder="Enter symbol"
        />
      </div>
      <div class="control-group">
        <label>Timeframe</label>
        <select v-model="localTimeframe" @change="updateTimeframe" class="timeframe-select">
          <option value="1m">1 Minute</option>
          <option value="5m">5 Minutes</option>
          <option value="15m">15 Minutes</option>
          <option value="1h">1 Hour</option>
          <option value="1D">1 Day</option>
          <option value="1W">1 Week</option>
        </select>
      </div>
      <div class="control-group">
        <label>Data Sources</label>
        <div v-if="dataSources?.length" class="data-list">
          <div v-for="ds in dataSources" :key="ds.id" class="data-item">{{ ds.name }}</div>
        </div>
        <div v-else class="data-empty">No sources</div>
      </div>
      <div class="control-group">
        <label>Data Sets</label>
        <div v-if="dataSets?.length" class="data-list">
          <div v-for="ds in dataSets" :key="ds.id" class="data-item">{{ ds.name }}</div>
        </div>
        <div v-else class="data-empty">No datasets</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { useWorkspaceStore } from '../stores/workspace'
import { api } from '../shared/api'

const props = defineProps<{
  symbol: string
  timeframe: string
}>()

const store = useWorkspaceStore()
const localSymbol = ref(props.symbol)
const localTimeframe = ref(props.timeframe)

const { data: dataSources } = useQuery({
  queryKey: ['datasources'],
  queryFn: api.getDataSources
})

const { data: dataSets } = useQuery({
  queryKey: ['datasets'],
  queryFn: api.getDataSets
})

watch(() => props.symbol, (val) => { localSymbol.value = val })
watch(() => props.timeframe, (val) => { localTimeframe.value = val })

const updateSymbol = () => {
  store.symbol = localSymbol.value
}

const updateTimeframe = () => {
  store.timeframe = localTimeframe.value
}
</script>

<style scoped>
.data-explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.panel-header {
  padding: 12px 16px;
  font-weight: 600;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.panel-content {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
}

.control-group {
  margin-bottom: 16px;
}

.control-group label {
  display: block;
  margin-bottom: 8px;
  color: #b0b0b0;
  font-size: 14px;
}

.symbol-input,
.timeframe-select {
  width: 100%;
  padding: 8px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  color: #e0e0e0;
  font-size: 14px;
}

.symbol-input:focus,
.timeframe-select:focus {
  outline: none;
  border-color: rgba(255, 255, 255, 0.3);
}

.data-list {
  max-height: 120px;
  overflow-y: auto;
}

.data-item {
  padding: 6px 8px;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 3px;
  margin-bottom: 4px;
  font-size: 13px;
  color: #d0d0d0;
}

.data-empty {
  padding: 8px;
  color: #666;
  font-size: 13px;
}
</style>
