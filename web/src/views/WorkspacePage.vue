<template>
  <div class="workspace-page">
    <div class="top-bar">
      <div class="top-bar-left">
        <span class="title">zQuant 工作区</span>
        <span v-if="health?.mode" class="mode-badge">{{ health.mode }}</span>
        <span v-if="health" :class="['health-indicator', health.status]" :title="health.last_error || ''">
          {{ health.status }}
        </span>
      </div>
      <div class="top-bar-controls">
        <button
          :class="['panel-btn', { active: rightPanel === 'data-explorer' }]"
          @click="store.rightPanel = 'data-explorer'"
        >
          数据
        </button>
        <button
          :class="['panel-btn', { active: rightPanel === 'governance-summary' }]"
          @click="store.rightPanel = 'governance-summary'"
        >
          治理
        </button>
      </div>
    </div>
    <div class="workspace-content">
      <div class="left-sidebar">
        <!-- Minimal sidebar -->
      </div>
      <div class="center-area">
        <div class="chart-panel">
          <PriceChartPanel :symbol="symbol" :timeframe="timeframe" />
        </div>
      </div>
      <div class="right-dock">
        <div v-if="rightPanel === 'data-explorer'" class="panel">
          <DataExplorerPanel :symbol="symbol" :timeframe="timeframe" />
        </div>
        <div v-if="rightPanel === 'governance-summary'" class="panel">
          <GovernanceSummaryPanel />
        </div>
      </div>
    </div>
    <div class="bottom-dock">
      <a-tabs v-model:activeKey="activeTab">
        <a-tab-pane key="jobs" tab="任务">
          <JobsTab />
        </a-tab-pane>
        <a-tab-pane key="logs" tab="日志">
          <LogsTab />
        </a-tab-pane>
      </a-tabs>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useQuery } from '@tanstack/vue-query'
import { useWorkspaceStore } from '../stores/workspace'
import { storeToRefs } from 'pinia'
import { api } from '../shared/api'
import PriceChartPanel from '../components/PriceChartPanel.vue'
import DataExplorerPanel from '../components/DataExplorerPanel.vue'
import GovernanceSummaryPanel from '../components/GovernanceSummaryPanel.vue'
import JobsTab from '../components/JobsTab.vue'
import LogsTab from '../components/LogsTab.vue'

const route = useRoute()
const router = useRouter()
const store = useWorkspaceStore()
const { symbol, timeframe, rightPanel } = storeToRefs(store)
const activeTab = ref('jobs')

const { data: health } = useQuery({
  queryKey: ['health'],
  queryFn: api.getHealth,
  refetchInterval: 10000
})

// Initialize from URL on mount
onMounted(() => {
  if (route.query.symbol) store.symbol = route.query.symbol as string
  if (route.query.timeframe) store.timeframe = route.query.timeframe as string
  if (route.query.right) store.rightPanel = route.query.right as string
  if (route.query.bottom) activeTab.value = route.query.bottom as string
})

// Sync store to URL
watch([symbol, timeframe, rightPanel, activeTab], () => {
  router.replace({
    query: {
      symbol: symbol.value,
      timeframe: timeframe.value,
      right: rightPanel.value,
      bottom: activeTab.value
    }
  })
})
</script>

<style scoped>
.workspace-page {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #0a0a0a;
  color: #e0e0e0;
}

.top-bar {
  height: 48px;
  background: rgba(20, 20, 20, 0.8);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
}

.top-bar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.title {
  font-weight: 500;
}

.mode-badge {
  padding: 2px 8px;
  background: rgba(38, 166, 154, 0.15);
  border: 1px solid rgba(38, 166, 154, 0.4);
  border-radius: 3px;
  color: #26a69a;
  font-size: 11px;
  text-transform: uppercase;
}

.health-indicator {
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 11px;
  text-transform: uppercase;
  font-weight: 600;
}

.health-indicator.healthy {
  background: rgba(56, 142, 60, 0.2);
  color: #66bb6a;
}

.health-indicator.degraded {
  background: rgba(245, 124, 0, 0.2);
  color: #ffa726;
}

.health-indicator.unhealthy {
  background: rgba(211, 47, 47, 0.2);
  color: #ef5350;
}

.top-bar-controls {
  display: flex;
  gap: 8px;
}

.panel-btn {
  padding: 6px 12px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  color: #b0b0b0;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.panel-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.2);
}

.panel-btn.active {
  background: rgba(38, 166, 154, 0.2);
  border-color: #26a69a;
  color: #26a69a;
}

.workspace-content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.left-sidebar {
  width: 48px;
  background: rgba(15, 15, 15, 0.9);
  border-right: 1px solid rgba(255, 255, 255, 0.1);
}

.center-area {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.chart-panel {
  flex: 1;
  background: rgba(10, 10, 10, 0.95);
  display: flex;
  align-items: center;
  justify-content: center;
}

.placeholder {
  color: #666;
  font-size: 18px;
}

.right-dock {
  width: 320px;
  background: rgba(15, 15, 15, 0.9);
  border-left: 1px solid rgba(255, 255, 255, 0.1);
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.panel {
  flex: 1;
  background: rgba(20, 20, 20, 0.8);
  padding: 16px;
}

.bottom-dock {
  height: 200px;
  margin-left: 48px;
  background: rgba(15, 15, 15, 0.9);
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}
</style>
