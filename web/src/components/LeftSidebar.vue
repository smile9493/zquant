<template>
  <div class="left-sidebar">
    <div class="sidebar-section">
      <div class="section-title">快速</div>
      <div
        v-for="symbol in quickList"
        :key="symbol"
        class="sidebar-item"
        @click="selectSymbol(symbol)"
      >
        {{ symbol }}
      </div>
    </div>
    <div class="sidebar-section">
      <div class="section-title">收藏</div>
      <div v-if="favorites.length === 0" class="sidebar-empty">无收藏</div>
      <div
        v-for="symbol in favorites"
        :key="symbol"
        class="sidebar-item-row"
      >
        <div class="sidebar-item" @click="selectSymbol(symbol)">
          {{ symbol }}
        </div>
        <button class="remove-btn" @click="removeFav(symbol)" title="移除">×</button>
      </div>
      <div class="add-favorite">
        <input
          v-model="newFavorite"
          @keyup.enter="addFav"
          placeholder="添加..."
          class="add-input"
        />
        <button @click="addFav" class="add-btn">+</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useWatchlistStore } from '../stores/watchlist'
import { storeToRefs } from 'pinia'

const workspaceStore = useWorkspaceStore()
const watchlistStore = useWatchlistStore()
const { favorites, quickList } = storeToRefs(watchlistStore)
const newFavorite = ref('')

const selectSymbol = (symbol: string) => {
  workspaceStore.symbol = symbol
}

const addFav = () => {
  const symbol = newFavorite.value.trim().toUpperCase()
  if (symbol) {
    watchlistStore.addFavorite(symbol)
    newFavorite.value = ''
  }
}

const removeFav = (symbol: string) => {
  watchlistStore.removeFavorite(symbol)
}
</script>

<style scoped>
.left-sidebar {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 12px 8px;
  overflow-y: auto;
}

.sidebar-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.section-title {
  font-size: 11px;
  color: #757575;
  text-transform: uppercase;
  margin-bottom: 4px;
  padding: 0 4px;
}

.sidebar-item {
  padding: 6px 8px;
  font-size: 12px;
  color: #b0b0b0;
  cursor: pointer;
  border-radius: 3px;
  transition: all 0.2s;
  flex: 1;
}

.sidebar-item:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #e0e0e0;
}

.sidebar-item-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

.remove-btn {
  padding: 2px 6px;
  background: rgba(239, 83, 80, 0.2);
  border: none;
  border-radius: 3px;
  color: #ef5350;
  font-size: 14px;
  cursor: pointer;
  opacity: 0.6;
  transition: opacity 0.2s;
}

.remove-btn:hover {
  opacity: 1;
}

.add-favorite {
  display: flex;
  gap: 4px;
  margin-top: 4px;
}

.add-input {
  flex: 1;
  padding: 4px 6px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 3px;
  color: #e0e0e0;
  font-size: 11px;
}

.add-input:focus {
  outline: none;
  border-color: #26a69a;
}

.add-btn {
  padding: 4px 8px;
  background: rgba(38, 166, 154, 0.2);
  border: 1px solid rgba(38, 166, 154, 0.4);
  border-radius: 3px;
  color: #26a69a;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.add-btn:hover {
  background: rgba(38, 166, 154, 0.3);
}

.sidebar-empty {
  padding: 6px 8px;
  font-size: 11px;
  color: #666;
}
</style>
