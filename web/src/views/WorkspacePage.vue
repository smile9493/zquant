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
      <div class="top-bar-center">
        <input
          v-model="store.symbol"
          class="symbol-input"
          placeholder="Symbol"
        />
        <select v-model="store.timeframe" class="timeframe-select">
          <option value="1m">1m</option>
          <option value="5m">5m</option>
          <option value="15m">15m</option>
          <option value="1h">1h</option>
          <option value="4h">4h</option>
          <option value="1D">1D</option>
          <option value="1W">1W</option>
        </select>
        <button class="refresh-btn" @click="handleRefresh" :disabled="isRefreshing">
          {{ isRefreshing ? '刷新中...' : '刷新' }}
        </button>
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
        <button
          :class="['panel-btn', { active: rightPanel === 'agent' }]"
          @click="store.rightPanel = 'agent'"
        >
          Agent
        </button>
      </div>
    </div>
    <div class="workspace-content">
      <div class="left-sidebar">
        <LeftSidebar />
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
        <div v-if="rightPanel === 'agent'" class="panel">
          <AgentPanel />
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
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { useWorkspaceStore } from '../stores/workspace'
import { useJobStore } from '../stores/jobs'
import { storeToRefs } from 'pinia'
import { api } from '../shared/api'
import PriceChartPanel from '../components/PriceChartPanel.vue'
import DataExplorerPanel from '../components/DataExplorerPanel.vue'
import GovernanceSummaryPanel from '../components/GovernanceSummaryPanel.vue'
import AgentPanel from '../components/AgentPanel.vue'
import JobsTab from '../components/JobsTab.vue'
import LogsTab from '../components/LogsTab.vue'
import LeftSidebar from '../components/LeftSidebar.vue'

const route = useRoute()
const router = useRouter()
const store = useWorkspaceStore()
const jobStore = useJobStore()
const queryClient = useQueryClient()
const { symbol, timeframe, rightPanel } = storeToRefs(store)
const { selectedJobId } = storeToRefs(jobStore)
const activeTab = ref('jobs')
const isRefreshing = ref(false)

const { data: health } = useQuery({
  queryKey: ['health'],
  queryFn: api.getHealth,
  refetchInterval: 10000
})

const handleRefresh = async () => {
  isRefreshing.value = true
  try {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['kline'] }),
      queryClient.invalidateQueries({ queryKey: ['jobs'] }),
      selectedJobId.value ? queryClient.invalidateQueries({ queryKey: ['logs', selectedJobId.value] }) : Promise.resolve()
    ])
  } finally {
    isRefreshing.value = false
  }
}

// Initialize from URL on mount
onMounted(() => {
  if (route.query.symbol) store.symbol = route.query.symbol as string
  if (route.query.timeframe) store.timeframe = route.query.timeframe as string
  if (route.query.right) {
    const panel = route.query.right as string
    if (panel === 'data-explorer' || panel === 'governance-summary' || panel === 'agent') {
      store.rightPanel = panel
    }
  }
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
  background: var(--zq-bg-page);
  color: var(--zq-text-primary);
}

.top-bar {
  height: var(--zq-height-topbar);
  background: var(--zq-bg-surface);
  border-bottom: 1px solid var(--zq-border-subtle);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--zq-space-4);
}

.top-bar-left {
  display: flex;
  align-items: center;
  gap: var(--zq-space-3);
}

.top-bar-center {
  display: flex;
  align-items: center;
  gap: var(--zq-space-2);
}

.symbol-input {
  width: 100px;
  padding: var(--zq-space-1) var(--zq-space-2);
  background: var(--zq-bg-input);
  border: 1px solid var(--zq-border-subtle);
  border-radius: var(--zq-radius-md);
  color: var(--zq-text-primary);
  font-size: var(--zq-font-size-md);
}

.symbol-input:focus {
  outline: none;
  border-color: var(--zq-color-primary);
}

.timeframe-select {
  padding: var(--zq-space-1) var(--zq-space-2);
  background: var(--zq-bg-input);
  border: 1px solid var(--zq-border-subtle);
  border-radius: var(--zq-radius-md);
  color: var(--zq-text-primary);
  font-size: var(--zq-font-size-md);
  cursor: pointer;
}

.timeframe-select:focus {
  outline: none;
  border-color: var(--zq-color-primary);
}

.refresh-btn {
  padding: var(--zq-space-1) var(--zq-space-3);
  background: var(--zq-primary-alpha-15);
  border: 1px solid var(--zq-primary-alpha-40);
  border-radius: var(--zq-radius-md);
  color: var(--zq-color-primary);
  font-size: var(--zq-font-size-md);
  cursor: pointer;
  transition: all var(--zq-transition-base);
}

.refresh-btn:hover:not(:disabled) {
  background: var(--zq-primary-alpha-25);
  border-color: var(--zq-color-primary);
}

.refresh-btn:disabled {
  opacity: var(--zq-opacity-disabled);
  cursor: not-allowed;
}

.title {
  font-weight: 500;
}

.mode-badge {
  padding: 2px var(--zq-space-2);
  background: var(--zq-primary-alpha-15);
  border: 1px solid var(--zq-primary-alpha-40);
  border-radius: var(--zq-radius-sm);
  color: var(--zq-color-primary);
  font-size: var(--zq-font-size-xs);
  text-transform: uppercase;
}

.health-indicator {
  padding: 2px var(--zq-space-2);
  border-radius: var(--zq-radius-sm);
  font-size: var(--zq-font-size-xs);
  text-transform: uppercase;
  font-weight: 600;
}

.health-indicator.healthy {
  background: var(--zq-success-alpha-20);
  color: var(--zq-text-success);
}

.health-indicator.degraded {
  background: var(--zq-warning-alpha-20);
  color: var(--zq-text-warning);
}

.health-indicator.unhealthy {
  background: var(--zq-error-alpha-20);
  color: var(--zq-text-error);
}

.top-bar-controls {
  display: flex;
  gap: var(--zq-space-2);
}

.panel-btn {
  padding: 6px var(--zq-space-3);
  background: var(--zq-bg-input);
  border: 1px solid var(--zq-border-subtle);
  border-radius: var(--zq-radius-md);
  color: var(--zq-text-secondary);
  font-size: var(--zq-font-size-md);
  cursor: pointer;
  transition: all var(--zq-transition-base);
}

.panel-btn:hover {
  background: var(--zq-bg-input-hover);
  border-color: var(--zq-border-emphasis);
}

.panel-btn.active {
  background: var(--zq-primary-alpha-20);
  border-color: var(--zq-color-primary);
  color: var(--zq-color-primary);
}

.workspace-content {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.left-sidebar {
  width: var(--zq-width-sidebar);
  background: var(--zq-bg-surface-elevated);
  border-right: 1px solid var(--zq-border-subtle);
}

.center-area {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.chart-panel {
  flex: 1;
  background: var(--zq-bg-surface-alt);
  display: flex;
  align-items: center;
  justify-content: center;
}

.placeholder {
  color: var(--zq-text-tertiary);
  font-size: var(--zq-font-size-xl);
}

.right-dock {
  width: var(--zq-width-dock);
  background: var(--zq-bg-surface-elevated);
  border-left: 1px solid var(--zq-border-subtle);
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.panel {
  flex: 1;
  background: var(--zq-bg-surface);
  padding: var(--zq-space-4);
}

.bottom-dock {
  height: var(--zq-height-bottom-dock);
  margin-left: var(--zq-width-sidebar);
  background: var(--zq-bg-surface-elevated);
  border-top: 1px solid var(--zq-border-subtle);
}
</style>
