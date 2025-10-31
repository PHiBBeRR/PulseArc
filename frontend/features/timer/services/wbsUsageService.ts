// Phase 4.3: WBS Usage Tracking Service
// Manages recent and favorite WBS codes with LocalStorage persistence

import type { WbsElement } from '@/shared/types/generated';

const STORAGE_KEY_RECENT = 'pulsarc_wbs_recent';
const STORAGE_KEY_FAVORITES = 'pulsarc_wbs_favorites';
const MAX_RECENT_CODES = 10;

export type RecentWbsEntry = {
  code: string;
  element: WbsElement;
  lastUsed: number; // timestamp
};

/**
 * WBS Usage Service
 *
 * Tracks frequently used WBS codes for improved user productivity:
 * - Recent codes: Last 10 used WBS codes (LRU order)
 * - Favorite codes: User-starred WBS codes (persist across sessions)
 *
 * Storage: LocalStorage for persistence
 */
export const WbsUsageService = {
  /**
   * Add WBS code to recent list
   * - Adds to front of list
   * - If already exists, moves to front
   * - Limits to MAX_RECENT_CODES
   */
  addRecentWbs(code: string, element: WbsElement): void {
    const recent = this.getRecentWbs();

    // Remove existing entry if present
    const filtered = recent.filter((entry) => entry.code !== code);

    // Add to front
    const newEntry: RecentWbsEntry = {
      code,
      element,
      lastUsed: Date.now(),
    };

    const updated = [newEntry, ...filtered].slice(0, MAX_RECENT_CODES);

    localStorage.setItem(STORAGE_KEY_RECENT, JSON.stringify(updated));
  },

  /**
   * Get recent WBS codes (up to 10, most recent first)
   */
  getRecentWbs(): RecentWbsEntry[] {
    try {
      const stored = localStorage.getItem(STORAGE_KEY_RECENT);
      if (!stored) return [];

      const parsed = JSON.parse(stored) as RecentWbsEntry[];
      return parsed.slice(0, MAX_RECENT_CODES);
    } catch (error) {
      console.error('Failed to parse recent WBS codes:', error);
      return [];
    }
  },

  /**
   * Get recent WBS elements (for autocomplete display)
   */
  getRecentElements(): WbsElement[] {
    return this.getRecentWbs().map((entry) => entry.element);
  },

  /**
   * Add WBS code to favorites
   */
  addFavorite(code: string): void {
    const favorites = this.getFavorites();
    if (!favorites.includes(code)) {
      const updated = [...favorites, code];
      localStorage.setItem(STORAGE_KEY_FAVORITES, JSON.stringify(updated));
    }
  },

  /**
   * Remove WBS code from favorites
   */
  removeFavorite(code: string): void {
    const favorites = this.getFavorites();
    const updated = favorites.filter((fav) => fav !== code);
    localStorage.setItem(STORAGE_KEY_FAVORITES, JSON.stringify(updated));
  },

  /**
   * Get all favorite WBS codes
   */
  getFavorites(): string[] {
    try {
      const stored = localStorage.getItem(STORAGE_KEY_FAVORITES);
      if (!stored) return [];

      return JSON.parse(stored) as string[];
    } catch (error) {
      console.error('Failed to parse favorite WBS codes:', error);
      return [];
    }
  },

  /**
   * Check if WBS code is favorited
   */
  isFavorite(code: string): boolean {
    return this.getFavorites().includes(code);
  },

  /**
   * Toggle favorite status
   */
  toggleFavorite(code: string): boolean {
    const isFav = this.isFavorite(code);
    if (isFav) {
      this.removeFavorite(code);
      return false;
    } else {
      this.addFavorite(code);
      return true;
    }
  },

  /**
   * Clear all recent codes (testing utility)
   */
  clearRecent(): void {
    localStorage.removeItem(STORAGE_KEY_RECENT);
  },

  /**
   * Clear all favorites (testing utility)
   */
  clearFavorites(): void {
    localStorage.removeItem(STORAGE_KEY_FAVORITES);
  },
};
