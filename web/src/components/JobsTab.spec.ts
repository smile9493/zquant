import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { VueQueryPlugin } from '@tanstack/vue-query'
import Antd from 'ant-design-vue'
import JobsTab from './JobsTab.vue'
import { useJobStore } from '../stores/jobs'

// Mock API
vi.mock('../shared/api', () => ({
  api: {
    getJobs: vi.fn(() => Promise.resolve([
      { job_id: 'job1', job_type: 'test', status: 'running', stop_requested: false, created_at: '2026-03-15T00:00:00Z', updated_at: '2026-03-15T00:00:00Z' }
    ])),
    stopJob: vi.fn(() => Promise.resolve()),
    retryJob: vi.fn(() => Promise.resolve({ job_id: 'job2' }))
  }
}))

// Mock WS client to avoid connection errors
vi.mock('../shared/ws', () => ({
  WsClient: vi.fn().mockImplementation(() => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    send: vi.fn(),
    onMessage: vi.fn(() => vi.fn()),
    onStateChange: vi.fn(() => vi.fn()),
    isConnected: vi.fn(() => false)
  }))
}))

describe('JobsTab component - WS disconnected optimistic UI', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('should show "已请求停止" when optimistic stop applied (WS disconnected)', async () => {
    const wrapper = mount(JobsTab, {
      global: {
        plugins: [createPinia(), [VueQueryPlugin, {}], Antd]
      }
    })

    const store = useJobStore()

    // Simulate WS disconnected, HTTP data loaded
    store.wsConnected = false
    store.setJobs([
      { job_id: 'job1', job_type: 'test', status: 'running', stop_requested: false, created_at: '2026-03-15T00:00:00Z', updated_at: '2026-03-15T00:00:00Z' } as any
    ])

    await wrapper.vm.$nextTick()

    // Apply optimistic stop
    store.applyOptimisticStop('job1')
    await wrapper.vm.$nextTick()

    // Verify DOM shows "已请求停止"
    expect(wrapper.text()).toContain('已请求停止')
  })

  it('should show optimistic new job with queued status after retry (WS disconnected)', async () => {
    const wrapper = mount(JobsTab, {
      global: {
        plugins: [createPinia(), [VueQueryPlugin, {}], Antd]
      }
    })

    const store = useJobStore()

    // Simulate WS disconnected
    store.wsConnected = false
    store.setJobs([
      { job_id: 'job1', job_type: 'test', status: 'error', stop_requested: false, created_at: '2026-03-15T00:00:00Z', updated_at: '2026-03-15T00:00:00Z' } as any
    ])

    await wrapper.vm.$nextTick()

    // Add optimistic new job (simulating retry success)
    store.addOptimisticJob({
      job_id: 'job2',
      job_type: 'test',
      status: 'queued',
      stop_requested: false,
      created_at: '2026-03-15T00:00:00Z',
      updated_at: '2026-03-15T00:00:00Z'
    } as any)

    await wrapper.vm.$nextTick()

    // Verify DOM shows new job
    expect(wrapper.text()).toContain('job2')
    expect(wrapper.text()).toContain('queued')
  })
})
