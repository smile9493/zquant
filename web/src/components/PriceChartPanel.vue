<template>
  <div ref="chartContainer" class="chart-container"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { createChart, CandlestickSeries, type IChartApi } from 'lightweight-charts'
import { api } from '../shared/api'

const props = defineProps<{
  symbol: string
  timeframe: string
}>()

const chartContainer = ref<HTMLElement>()
let chart: IChartApi | null = null
let candlestickSeries: any = null

const { data: klineData } = useQuery({
  queryKey: ['kline', props.symbol, props.timeframe],
  queryFn: () => api.getKline(props.symbol, props.timeframe)
})

onMounted(() => {
  if (!chartContainer.value) return

  // Read colors from CSS variables
  const styles = getComputedStyle(document.documentElement)
  const textColor = styles.getPropertyValue('--zq-text-primary').trim()
  const gridColor = styles.getPropertyValue('--zq-border-subtle').trim()
  const upColor = styles.getPropertyValue('--zq-color-primary').trim()
  const downColor = styles.getPropertyValue('--zq-color-danger').trim()

  chart = createChart(chartContainer.value, {
    layout: {
      background: { color: 'transparent' },
      textColor,
    },
    grid: {
      vertLines: { color: gridColor },
      horzLines: { color: gridColor },
    },
    width: chartContainer.value.clientWidth,
    height: chartContainer.value.clientHeight,
  })

  candlestickSeries = chart.addSeries(CandlestickSeries, {
    upColor,
    downColor,
    borderVisible: false,
    wickUpColor: upColor,
    wickDownColor: downColor,
  })

  const handleResize = () => {
    if (chart && chartContainer.value) {
      chart.applyOptions({
        width: chartContainer.value.clientWidth,
        height: chartContainer.value.clientHeight,
      })
    }
  }

  window.addEventListener('resize', handleResize)
  onUnmounted(() => {
    window.removeEventListener('resize', handleResize)
    chart?.remove()
  })
})

watch(klineData, (data) => {
  if (data && candlestickSeries) {
    candlestickSeries.setData(data)
    chart?.timeScale().fitContent()
  }
})
</script>

<style scoped>
.chart-container {
  width: 100%;
  height: 100%;
}
</style>
