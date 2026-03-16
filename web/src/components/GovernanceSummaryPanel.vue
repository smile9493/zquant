<template>
  <div class="governance-summary">
    <div class="panel-header">治理概览</div>
    <div class="panel-content">
      <div v-if="isLoading" class="status-item">加载中...</div>
      <div v-else-if="error" class="status-item error">健康检查失败</div>
      <div v-else>
        <div class="status-item">
          状态: <span :class="['status-badge', data?.status]">{{ data?.status || '未知' }}</span>
        </div>
        <div v-if="data?.mode" class="status-item">模式: {{ data.mode }}</div>
        <div v-if="data?.last_error" class="status-item error-text">最近错误: {{ data.last_error }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import { api } from '../shared/api'

const { data, isLoading, error } = useQuery({
  queryKey: ['health'],
  queryFn: api.getHealth,
  refetchInterval: 10000
})
</script>

<style scoped>
.governance-summary {
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

.status-item {
  padding: var(--zq-space-2) 0;
  color: var(--zq-text-secondary);
}

.status-item.error {
  color: var(--zq-text-error);
}

.error-text {
  color: var(--zq-text-error);
  font-size: var(--zq-font-size-sm);
}

.status-badge {
  padding: var(--zq-space-05) var(--zq-space-2);
  border-radius: var(--zq-radius-sm);
  font-size: var(--zq-font-size-sm);
  font-weight: 600;
}

.status-badge.healthy {
  background: var(--zq-color-success);
  color: var(--zq-color-white);
}

.status-badge.degraded {
  background: var(--zq-color-warning-dark);
  color: var(--zq-color-white);
}

.status-badge.unhealthy {
  background: var(--zq-color-error);
  color: var(--zq-color-white);
}
</style>
