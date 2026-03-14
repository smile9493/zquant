import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

const STORAGE_KEY = 'zquant_watchlist_v1'

interface WatchlistData {
  favorites: string[]
}

const loadFromStorage = (): WatchlistData => {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored) {
    try {
      return JSON.parse(stored)
    } catch {
      return { favorites: [] }
    }
  }
  return { favorites: [] }
}

const saveToStorage = (data: WatchlistData) => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
}

export const useWatchlistStore = defineStore('watchlist', () => {
  const favorites = ref<string[]>(loadFromStorage().favorites)
  const quickList = ref<string[]>(['AAPL', 'GOOGL', 'MSFT', 'TSLA', 'AMZN'])

  watch(favorites, (val) => {
    saveToStorage({ favorites: val })
  }, { deep: true })

  const addFavorite = (symbol: string) => {
    if (!favorites.value.includes(symbol)) {
      favorites.value.push(symbol)
    }
  }

  const removeFavorite = (symbol: string) => {
    favorites.value = favorites.value.filter(s => s !== symbol)
  }

  const isFavorite = (symbol: string) => {
    return favorites.value.includes(symbol)
  }

  return {
    favorites,
    quickList,
    addFavorite,
    removeFavorite,
    isFavorite
  }
})
