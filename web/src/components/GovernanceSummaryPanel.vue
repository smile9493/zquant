<template>
  <div class="governance-summary">
    <div class="panel-header">Governance Summary</div>
    <div class="panel-content">
      <div v-if="isLoading" class="status-item">Loading...</div>
      <div v-else-if="error" class="status-item error">Health check failed</div>
      <div v-else>
        <div class="status-item">
          Status: <span :class="['status-badge', data?.status]">{{ data?.status || 'unknown' }}</span>
        </div>
        <div v-if="data?.mode" class="status-item">Mode: {{ data.mode }}</div>
        <div v-if="data?.last_error" class="status-item error-text">Last Error: {{ data.last_error }}</div>
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
  padding: 12px 16px;
  font-weight: 600;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.panel-content {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
}

.status-item {
  padding: 8px 0;
  color: #b0b0b0;
}

.status-item.error {
  color: #ef5350;
}

.error-text {
  color: #ff6b6b;
  font-size: 12px;
}

.status-badge {
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 12px;
  font-weight: 600;
}

.status-badge.healthy {
  background: #388e3c;
  color: #fff;
}

.status-badge.degraded {
  background: #f57c00;
  color: #fff;
}

.status-badge.unhealthy {
  background: #d32f2f;
  color: #fff;
}
</style>
