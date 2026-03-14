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
  cursor: pointer;
  transition: all 0.2s;
}

.data-item:hover {
  background: rgba(255, 255, 255, 0.08);
}

.data-item.selected {
  background: rgba(38, 166, 154, 0.2);
  border: 1px solid rgba(38, 166, 154, 0.4);
  color: #26a69a;
}

.data-empty,
.data-loading,
.data-error {
  padding: 8px;
  font-size: 13px;
}

.data-empty {
  color: #666;
}

.data-loading {
  color: #9e9e9e;
}

.data-error {
  color: #ef5350;
}
</style>
