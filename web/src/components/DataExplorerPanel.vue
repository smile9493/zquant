<template>
  <div class="data-explorer">
    <div class="panel-header">数据浏览器</div>
    <div class="panel-content">
      <div class="control-group">
        <label>代码</label>
        <input
          v-model="localSymbol"
          @change="updateSymbol"
          class="symbol-input"
          placeholder="输入代码"
        />
      </div>
      <div class="control-group">
        <label>时间周期</label>
        <select v-model="localTimeframe" @change="updateTimeframe" class="timeframe-select">
          <option value="1m">1分钟</option>
          <option value="5m">5分钟</option>
          <option value="15m">15分钟</option>
          <option value="1h">1小时</option>
          <option value="1D">1天</option>
          <option value="1W">1周</option>
        </select>
      </div>
      <div class="control-group">
        <label>数据源</label>
        <div v-if="loadingDataSources" class="data-loading">加载中...</div>
        <div v-else-if="errorDataSources" class="data-error">加载失败</div>
        <div v-else-if="dataSources?.length" class="data-list">
          <div
            v-for="ds in dataSources"
            :key="ds.id"
            :class="['data-item', { selected: ds.id === selectedDataSourceId }]"
            @click="selectDataSource(ds.id)"
          >
            {{ ds.name }}
          </div>
        </div>
        <div v-else class="data-empty">暂无数据源</div>
      </div>
      <div class="control-group">
        <label>数据集</label>
        <div v-if="loadingDataSets" class="data-loading">加载中...</div>
        <div v-else-if="errorDataSets" class="data-error">加载失败</div>
        <div v-else-if="dataSets?.length" class="data-list">
          <div
            v-for="ds in dataSets"
            :key="ds.id"
            :class="['data-item', { selected: ds.id === selectedDataSetId }]"
            @click="selectDataSet(ds.id)"
          >
            {{ ds.name }}
          </div>
        </div>
        <div v-else class="data-empty">暂无数据集</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { useWorkspaceStore } from '../stores/workspace'
import { useDataSourceStore } from '../stores/datasource'
import { storeToRefs } from 'pinia'
import { api } from '../shared/api'

const props = defineProps<{
  symbol: string
  timeframe: string
}>()

const store = useWorkspaceStore()
const dataSourceStore = useDataSourceStore()
const { selectedDataSourceId, selectedDataSetId } = storeToRefs(dataSourceStore)
const localSymbol = ref(props.symbol)
const localTimeframe = ref(props.timeframe)

const { data: dataSources, isLoading: loadingDataSources, error: errorDataSources } = useQuery({
  queryKey: ['datasources'],
  queryFn: api.getDataSources
})

const { data: dataSets, isLoading: loadingDataSets, error: errorDataSets } = useQuery({
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

const selectDataSource = (id: string) => {
  dataSourceStore.selectDataSource(id)
}

const selectDataSet = (id: string) => {
  dataSourceStore.selectDataSet(id)
}
</script>

<style scoped>
.data-explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.panel-header {
  padding: var(--zq-space-3) var(--zq-space-4);
  font-weight: 600;
  border-bottom: var(--zq-border-width-1) solid var(--zq-border-subtle);
}

.panel-content {
  flex: 1;
  padding: var(--zq-space-4);
  overflow-y: auto;
}

.control-group {
  margin-bottom: var(--zq-space-4);
}

.control-group label {
  display: block;
  margin-bottom: var(--zq-space-2);
  color: var(--zq-text-secondary);
  font-size: var(--zq-font-size-base);
}

.symbol-input,
.timeframe-select {
  width: 100%;
  padding: var(--zq-space-2);
  background: var(--zq-bg-input);
  border: var(--zq-border-width-1) solid var(--zq-border-subtle);
  border-radius: var(--zq-radius-md);
  color: var(--zq-text-primary);
  font-size: var(--zq-font-size-base);
}

.symbol-input:focus,
.timeframe-select:focus {
  outline: none;
  border-color: var(--zq-border-emphasis);
}

.data-list {
  max-height: 120px;
  overflow-y: auto;
}

.data-item {
  padding: var(--zq-space-15) var(--zq-space-2);
  background: var(--zq-bg-item);
  border-radius: var(--zq-radius-sm);
  margin-bottom: var(--zq-space-1);
  font-size: var(--zq-font-size-md);
  color: var(--zq-text-primary);
  cursor: pointer;
  transition: all var(--zq-transition-base);
}

.data-item:hover {
  background: var(--zq-bg-input-hover);
}

.data-item.selected {
  background: var(--zq-primary-alpha-20);
  border: var(--zq-border-width-1) solid var(--zq-primary-alpha-40);
  color: var(--zq-color-primary);
}

.data-empty,
.data-loading,
.data-error {
  padding: var(--zq-space-2);
  font-size: var(--zq-font-size-md);
}

.data-empty {
  color: var(--zq-text-tertiary);
}

.data-loading {
  color: var(--zq-color-gray-500);
}

.data-error {
  color: var(--zq-text-error);
}
</style>
