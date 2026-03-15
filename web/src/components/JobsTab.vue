<template>
  <div class="jobs-tab">
    <div class="toolbar">
      <a-button size="small" @click="refetch" :loading="isFetching">刷新</a-button>
      <span v-if="selectedJobId" class="selected">已选中：{{ selectedJobId }}</span>
    </div>

    <div v-if="isLoading" class="status">加载中...</div>
    <div v-else-if="error" class="status error">加载任务失败</div>
    <div v-else-if="!data?.length" class="status">暂无任务</div>
    <div v-else class="job-list">
      <div
        v-for="job in data"
        :key="job.job_id"
        :class="['job-item', { selected: job.job_id === selectedJobId }]"
        @click="selectJob(job.job_id)"
      >
        <div class="job-main">
          <div class="job-id">{{ job.job_id }}</div>
          <div class="job-meta">
            <span class="job-type">{{ job.job_type }}</span>
            <span class="job-time">更新：{{ formatTime(job.updated_at) }}</span>
            <span v-if="job.stop_requested" class="job-stop">已请求停止</span>
            <span v-if="(job as any)._retrying" class="job-retrying">重试中</span>
          </div>
        </div>

        <div class="job-right">
          <span :class="['job-status', `st-${String(job.status)}`]">{{ job.status }}</span>
          <div class="actions" @click.stop>
            <a-button size="small" danger @click="openStop(job.job_id)" :disabled="pendingActions.has(job.job_id)">停止</a-button>
            <a-popconfirm
              title="确认重试该任务？"
              ok-text="重试"
              cancel-text="取消"
              @confirm="retry(job.job_id)"
            >
              <a-button size="small" :disabled="pendingActions.has(job.job_id)">重试</a-button>
            </a-popconfirm>
          </div>
        </div>
      </div>
    </div>

    <a-modal
      v-model:open="stopModalOpen"
      title="停止任务"
      ok-text="确认停止"
      cancel-text="取消"
      :confirm-loading="stopMutation.isPending.value"
      @ok="confirmStop"
    >
      <div class="modal-body">
        <div class="modal-hint">任务：{{ stopTargetJobId }}</div>
        <a-input
          v-model:value="stopReason"
          placeholder="可选：停止原因"
          allow-clear
        />
      </div>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { message } from 'ant-design-vue'
import { ref, onMounted, onUnmounted, computed, watchEffect } from 'vue'
import { api } from '../shared/api'
import { useJobStore } from '../stores/jobs'
import { storeToRefs } from 'pinia'

const jobStore = useJobStore()
const { selectedJobId, wsConnected, displayJobs: wsJobs, pendingActions } = storeToRefs(jobStore)
const queryClient = useQueryClient()

const {
  data: httpData,
  isLoading,
  error,
  refetch,
  isFetching
} = useQuery({
  queryKey: ['jobs'],
  queryFn: api.getJobs,
  refetchInterval: () => wsConnected.value ? 30000 : 5000
})

watchEffect(() => {
  if (!wsConnected.value && httpData.value) {
    jobStore.setJobs(httpData.value)
  }
})

const data = computed(() => wsJobs.value)

let unsubscribe: (() => void) | null = null

onMounted(() => {
  unsubscribe = jobStore.initWs()
})

onUnmounted(() => {
  if (unsubscribe) {
    unsubscribe()
  }
  jobStore.disconnectWs()
})

const selectJob = (jobId: string) => {
  jobStore.selectJob(jobId)
}

const stopModalOpen = ref(false)
const stopTargetJobId = ref<string | null>(null)
const stopReason = ref('')

const openStop = (jobId: string) => {
  stopTargetJobId.value = jobId
  stopReason.value = ''
  stopModalOpen.value = true
}

const stopMutation = useMutation({
  mutationFn: async (params: { jobId: string; reason?: string }) => {
    await api.stopJob(params.jobId, params.reason)
  },
  onMutate: async (params) => {
    jobStore.applyOptimisticStop(params.jobId)
  },
  onSuccess: async () => {
    message.success('已请求停止')
    await queryClient.invalidateQueries({ queryKey: ['jobs'] })
    await queryClient.invalidateQueries({ queryKey: ['logs'] })
  },
  onError: (err, params) => {
    jobStore.clearOptimistic(params.jobId)
    message.error(err instanceof Error ? err.message : '停止失败')
  }
})

const confirmStop = async () => {
  if (!stopTargetJobId.value) return
  await stopMutation.mutateAsync({
    jobId: stopTargetJobId.value,
    reason: stopReason.value.trim() ? stopReason.value.trim() : undefined
  })
  stopModalOpen.value = false
}

const retryMutation = useMutation({
  mutationFn: async (jobId: string) => api.retryJob(jobId),
  onMutate: async (jobId) => {
    jobStore.applyOptimisticRetry(jobId)
  },
  onSuccess: async (data, jobId) => {
    jobStore.clearOptimistic(jobId)
    const originalJob = wsJobs.value.find(j => j.job_id === jobId) || httpData.value?.find(j => j.job_id === jobId)
    if (data.job_id && originalJob) {
      jobStore.addOptimisticJob({
        job_id: data.job_id,
        job_type: originalJob.job_type,
        status: 'queued',
        stop_requested: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      } as any)
    }
    message.success(`已重试，新的任务：${data.job_id}`)
    await queryClient.invalidateQueries({ queryKey: ['jobs'] })
  },
  onError: (err, jobId) => {
    jobStore.clearOptimistic(jobId)
    message.error(err instanceof Error ? err.message : '重试失败')
  }
})

const retry = async (jobId: string) => {
  await retryMutation.mutateAsync(jobId)
}

const formatTime = (iso: string) => {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return iso
  return date.toLocaleString()
}
</script>

<style scoped>
.jobs-tab {
  padding: 16px;
  height: 100%;
  overflow-y: auto;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.selected {
  color: #9fa8da;
  font-size: 12px;
}

.status {
  padding: 8px;
  color: #b0b0b0;
}

.status.error {
  color: #ef5350;
}

.job-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.job-item {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 10px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 4px;
  border: 1px solid transparent;
  cursor: pointer;
}

.job-item:hover {
  border-color: rgba(255, 255, 255, 0.12);
}

.job-item.selected {
  border-color: rgba(38, 166, 154, 0.6);
  background: rgba(38, 166, 154, 0.08);
}

.job-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.job-id {
  color: #e0e0e0;
  font-family: monospace;
  font-size: 12px;
  word-break: break-all;
}

.job-meta {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  color: #9e9e9e;
  font-size: 12px;
}

.job-status {
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 12px;
  color: #fff;
  text-transform: uppercase;
}

.job-right {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
  flex-shrink: 0;
}

.actions {
  display: flex;
  gap: 8px;
}

.job-stop {
  color: #ffb74d;
}

.job-retrying {
  color: #64b5f6;
}

.job-status.st-running {
  background: #1976d2;
}

.job-status.st-done {
  background: #388e3c;
}

.job-status.st-error {
  background: #d32f2f;
}

.job-status.st-queued {
  background: #757575;
}

.job-status.st-stopped {
  background: #616161;
}

.job-status.st-reaped {
  background: #455a64;
}

.modal-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.modal-hint {
  color: #b0b0b0;
  font-size: 12px;
}
</style>
