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
const { selectedJobId } = storeToRefs(store)

const { data, isLoading, error, refetch, isFetching } = useQuery({
  queryKey: computed(() => ['logs', selectedJobId.value]),
  queryFn: () => api.getJobLogs(selectedJobId.value!),
  refetchInterval: 5000,
  enabled: computed(() => !!selectedJobId.value)
})
</script>

<style scoped>
.logs-tab {
  padding: 16px;
  height: 100%;
  overflow-y: auto;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.title {
  display: flex;
  align-items: baseline;
  gap: 10px;
  min-width: 0;
}

.label {
  color: #e0e0e0;
  font-weight: 600;
}

.job {
  font-size: 12px;
  color: #b0b0b0;
  font-family: monospace;
  word-break: break-all;
}

.job.empty {
  font-family: inherit;
  color: #757575;
}

.status {
  padding: 8px;
  color: #b0b0b0;
}

.status.error {
  color: #ef5350;
}

.log-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.log-item {
  padding: 6px 8px;
  font-family: monospace;
  font-size: 13px;
  display: flex;
  gap: 12px;
}

.log-item.info {
  color: #90caf9;
}

.log-item.warn {
  color: #ffb74d;
}

.log-item.error {
  color: #ef5350;
}

.log-time {
  color: #757575;
  flex-shrink: 0;
}

.log-message {
  flex: 1;
}
</style>
