<template>
  <div class="agent-panel">
    <div class="panel-header">
      <h3>Agent</h3>
      <span v-if="session" :class="['status-badge', session.status]">
        {{ session.status }}
      </span>
    </div>

    <div v-if="isLoading" class="loading-state">
      加载中...
    </div>

    <div v-else-if="error" class="error-state">
      <p>{{ error }}</p>
    </div>

    <div v-else-if="!session" class="empty-state">
      <p>暂无 Agent 会话</p>
    </div>

    <div v-else class="session-content">
      <div class="context-section">
        <h4>上下文</h4>
        <div class="context-item" v-if="session.symbol">
          <span class="label">Symbol:</span>
          <span class="value">{{ session.symbol }}</span>
        </div>
        <div class="context-item" v-if="session.timeframe">
          <span class="label">Timeframe:</span>
          <span class="value">{{ session.timeframe }}</span>
        </div>
        <div class="context-item" v-if="session.dataset_id">
          <span class="label">Dataset:</span>
          <span class="value">{{ session.dataset_id }}</span>
        </div>
        <div class="context-item" v-if="session.job_id">
          <span class="label">Job:</span>
          <span class="value">{{ session.job_id }}</span>
        </div>
      </div>

      <div v-if="session.last_output" class="output-section">
        <h4>最近输出</h4>
        <div class="output-content">{{ session.last_output }}</div>
      </div>

      <div v-if="session.error_message" class="error-section">
        <h4>错误信息</h4>
        <div class="error-content">{{ session.error_message }}</div>
      </div>

      <div class="meta-section">
        <span class="meta-label">最后更新:</span>
        <span class="meta-value">{{ formatTime(session.last_updated) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import { api } from '../shared/api'

const { data: session, isLoading, error } = useQuery({
  queryKey: ['agent-session'],
  queryFn: api.getAgentSession,
  refetchInterval: 10000
})

const formatTime = (timestamp: string) => {
  return new Date(timestamp).toLocaleString('zh-CN')
}
</script>

<style scoped>
.agent-panel {
  display: flex;
  flex-direction: column;
  gap: var(--zq-space-4);
  height: 100%;
  overflow-y: auto;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-header h3 {
  font-size: var(--zq-font-size-base);
  font-weight: 600;
  color: var(--zq-text-primary);
}

.status-badge {
  padding: var(--zq-space-05) var(--zq-space-2);
  border-radius: var(--zq-radius-sm);
  font-size: var(--zq-font-size-xs);
  text-transform: uppercase;
  font-weight: 600;
}

.status-badge.idle {
  background: var(--zq-idle-alpha-20);
  color: var(--zq-color-gray-500);
}

.status-badge.running {
  background: var(--zq-running-alpha-20);
  color: var(--zq-text-running);
}

.status-badge.success {
  background: var(--zq-success-alpha-20);
  color: var(--zq-text-success);
}

.status-badge.error {
  background: var(--zq-error-alpha-20);
  color: var(--zq-text-error);
}

.loading-state,
.error-state,
.empty-state {
  padding: var(--zq-space-6);
  text-align: center;
  color: var(--zq-text-muted);
  font-size: var(--zq-font-size-md);
}

.error-state {
  color: var(--zq-text-error);
}

.session-content {
  display: flex;
  flex-direction: column;
  gap: var(--zq-space-4);
}

.context-section,
.output-section,
.error-section {
  display: flex;
  flex-direction: column;
  gap: var(--zq-space-2);
}

.context-section h4,
.output-section h4,
.error-section h4 {
  font-size: var(--zq-font-size-sm);
  font-weight: 600;
  color: var(--zq-text-secondary);
  text-transform: uppercase;
}

.context-item {
  display: flex;
  justify-content: space-between;
  font-size: var(--zq-font-size-md);
}

.context-item .label {
  color: var(--zq-text-muted);
}

.context-item .value {
  color: var(--zq-text-primary);
  font-family: 'JetBrains Mono', monospace;
}

.output-content,
.error-content {
  padding: var(--zq-space-3);
  background: var(--zq-bg-code);
  border: var(--zq-border-width-1) solid var(--zq-border-subtle);
  border-radius: var(--zq-radius-md);
  font-size: var(--zq-font-size-sm);
  font-family: 'JetBrains Mono', monospace;
  color: var(--zq-text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

.error-content {
  color: var(--zq-text-error);
  border-color: var(--zq-border-error);
}

.meta-section {
  display: flex;
  gap: var(--zq-space-2);
  font-size: var(--zq-font-size-xs);
  color: var(--zq-text-tertiary);
}

.meta-label {
  color: var(--zq-text-muted);
}

.meta-value {
  color: var(--zq-text-secondary);
}
</style>
