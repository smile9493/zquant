<template>
  <div class="logs-tab">
    <div v-if="isLoading" class="status">Loading...</div>
    <div v-else-if="error" class="status error">Error loading logs</div>
    <div v-else-if="!data?.length" class="status">No logs available</div>
    <div v-else class="log-list">
      <div v-for="(log, i) in data" :key="i" :class="['log-item', log.level]">
        <span class="log-time">{{ log.timestamp }}</span>
        <span class="log-message">{{ log.message }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import { api } from '../shared/api'
import { useWorkspaceStore } from '../stores/workspace'
import { storeToRefs } from 'pinia'

const store = useWorkspaceStore()
const { selectedJobId } = storeToRefs(store)

const { data, isLoading, error } = useQuery({
  queryKey: ['logs', selectedJobId],
  queryFn: () => api.getJobLogs(selectedJobId.value!),
  refetchInterval: 5000,
  enabled: !!selectedJobId.value
})
</script>

<style scoped>
.logs-tab {
  padding: 16px;
  height: 100%;
  overflow-y: auto;
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
