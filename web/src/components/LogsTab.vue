<template>
  <div class="logs-tab">
    <div class="toolbar">
      <div class="title">
        <span class="label">日志</span>
        <span v-if="selectedJobId" class="job">任务：{{ selectedJobId }}</span>
        <span v-else class="job empty">未选择任务</span>
      </div>
      <a-button size="small" @click="refetch" :disabled="!selectedJobId" :loading="isFetching">
        刷新
      </a-button>
    </div>

    <div v-if="!selectedJobId" class="status">请选择一个任务查看日志</div>
    <div v-else-if="isLoading" class="status">加载中...</div>
    <div v-else-if="error" class="status error">加载日志失败</div>
    <div v-else-if="!data?.length" class="status">暂无日志</div>
    <div v-else class="log-list">
      <div v-for="(log, i) in data" :key="i" :class="['log-item', log.level]">
        <span class="log-time">{{ log.timestamp }}</span>
        <span class="log-message">{{ log.message }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { api } from '../shared/api'
import { useJobStore } from '../stores/jobs'
import { storeToRefs } from 'pinia'

const store = useJobStore()
const { selectedJobId, wsConnected, logs: wsLogs } = storeToRefs(store)

const { data: httpData, isLoading, error, refetch, isFetching } = useQuery({
  queryKey: computed(() => ['logs', selectedJobId.value]),
  queryFn: () => api.getJobLogs(selectedJobId.value!),
  refetchInterval: () => wsConnected.value ? false : 5000,
  enabled: computed(() => !!selectedJobId.value)
})

const data = computed(() => {
  if (wsConnected.value && selectedJobId.value && wsLogs.value.has(selectedJobId.value)) {
    return wsLogs.value.get(selectedJobId.value)
  }
  return httpData.value
})
</script>

<style scoped>
.logs-tab {
  padding: var(--zq-space-4);
  height: 100%;
  overflow-y: auto;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--zq-space-3);
  margin-bottom: var(--zq-space-3);
}

.title {
  display: flex;
  align-items: baseline;
  gap: var(--zq-space-3);
  min-width: 0;
}

.label {
  color: var(--zq-text-primary);
  font-weight: 600;
}

.job {
  font-size: var(--zq-font-size-sm);
  color: var(--zq-text-secondary);
  font-family: monospace;
  word-break: break-all;
}

.job.empty {
  font-family: inherit;
  color: var(--zq-color-gray-600);
}

.status {
  padding: var(--zq-space-2);
  color: var(--zq-text-secondary);
}

.status.error {
  color: var(--zq-text-error);
}

.log-list {
  display: flex;
  flex-direction: column;
  gap: var(--zq-space-1);
}

.log-item {
  padding: 6px var(--zq-space-2);
  font-family: monospace;
  font-size: var(--zq-font-size-md);
  display: flex;
  gap: var(--zq-space-3);
}

.log-item.info {
  color: var(--zq-text-info);
}

.log-item.warn {
  color: var(--zq-text-warning);
}

.log-item.error {
  color: var(--zq-text-error);
}

.log-time {
  color: var(--zq-color-gray-600);
  flex-shrink: 0;
}

.log-message {
  flex: 1;
}
</style>
