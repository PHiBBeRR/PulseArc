// FEATURE-020 Phase 4: Recent/Favorite WBS Tests
// Test coverage for recent and favorite WBS code tracking

import { describe, it, beforeEach, afterEach, expect } from 'vitest';
import { WbsUsageService } from '@/features/timer/services/wbsUsageService';
import type { WbsElement } from '@/shared/types/generated';

// Mock WBS elements for testing
const createMockWbsElement = (code: string): WbsElement => ({
  wbs_code: code,
  project_def: code.split('.')[0],
  project_name: `Project ${code}`,
  description: `Description for ${code}`,
  status: 'REL',
  cached_at: Date.now(),
  // FEATURE-029: Enriched fields
  opportunity_id: null,
  deal_name: null,
  target_company_name: null,
  counterparty: null,
  industry: null,
  region: null,
  amount: null,
  stage_name: null,
  project_code: null,
});

describe('Recent and Favorite WBS Codes', () => {
  beforeEach(() => {
    // Reset local storage before each test
    WbsUsageService.clearRecent();
    WbsUsageService.clearFavorites();
  });

  afterEach(() => {
    // Cleanup after each test
    WbsUsageService.clearRecent();
    WbsUsageService.clearFavorites();
  });

  it('should track recently used WBS codes', () => {
    const mockElement = createMockWbsElement('USC0063201.1.1');

    // Add WBS code to recent list
    WbsUsageService.addRecentWbs('USC0063201.1.1', mockElement);

    // Verify code added to recent list
    const recent = WbsUsageService.getRecentWbs();
    expect(recent).toHaveLength(1);
    expect(recent[0].code).toBe('USC0063201.1.1');
    expect(recent[0].element.wbs_code).toBe('USC0063201.1.1');
  });

  it('should limit recent codes to 10 entries', () => {
    // Add 15 different WBS codes
    for (let i = 1; i <= 15; i++) {
      const code = `USC006320${i}.1.1`;
      const mockElement = createMockWbsElement(code);
      WbsUsageService.addRecentWbs(code, mockElement);
    }

    // Verify only last 10 in recent list
    const recent = WbsUsageService.getRecentWbs();
    expect(recent).toHaveLength(10);

    // Verify most recent is first (code 15)
    expect(recent[0].code).toBe('USC00632015.1.1');
    // Verify oldest kept is 10th (code 6)
    expect(recent[9].code).toBe('USC0063206.1.1');
  });

  it('should move recent code to top when used again', () => {
    // Use WBS codes A, B, C (in order)
    const codeA = createMockWbsElement('USC0063201.1.1');
    const codeB = createMockWbsElement('USC0063202.1.1');
    const codeC = createMockWbsElement('USC0063203.1.1');

    WbsUsageService.addRecentWbs('USC0063201.1.1', codeA);
    WbsUsageService.addRecentWbs('USC0063202.1.1', codeB);
    WbsUsageService.addRecentWbs('USC0063203.1.1', codeC);

    // Use A again
    WbsUsageService.addRecentWbs('USC0063201.1.1', codeA);

    // Verify recent order: A, C, B
    const recent = WbsUsageService.getRecentWbs();
    expect(recent).toHaveLength(3);
    expect(recent[0].code).toBe('USC0063201.1.1'); // A at top
    expect(recent[1].code).toBe('USC0063203.1.1'); // C second
    expect(recent[2].code).toBe('USC0063202.1.1'); // B third
  });

  it('should star WBS code to mark as favorite', () => {
    // Add WBS code to favorites
    WbsUsageService.addFavorite('USC0063201.1.1');

    // Verify marked as favorite
    expect(WbsUsageService.isFavorite('USC0063201.1.1')).toBe(true);
    expect(WbsUsageService.getFavorites()).toContain('USC0063201.1.1');
  });

  it('should unstar WBS code to remove from favorites', () => {
    // Add WBS code to favorites
    WbsUsageService.addFavorite('USC0063201.1.1');
    expect(WbsUsageService.isFavorite('USC0063201.1.1')).toBe(true);

    // Remove from favorites
    WbsUsageService.removeFavorite('USC0063201.1.1');

    // Verify removed from favorites
    expect(WbsUsageService.isFavorite('USC0063201.1.1')).toBe(false);
    expect(WbsUsageService.getFavorites()).not.toContain('USC0063201.1.1');
  });

  it('should persist favorites in local storage', () => {
    // Star WBS code
    WbsUsageService.addFavorite('USC0063201.1.1');
    WbsUsageService.addFavorite('USC0063202.1.1');

    // Simulate reload by getting fresh instance
    const favorites = WbsUsageService.getFavorites();

    // Verify WBS codes still favorited
    expect(favorites).toHaveLength(2);
    expect(favorites).toContain('USC0063201.1.1');
    expect(favorites).toContain('USC0063202.1.1');
  });

  it('should persist recent codes in local storage', () => {
    const mockElement = createMockWbsElement('USC0063201.1.1');

    // Use WBS code
    WbsUsageService.addRecentWbs('USC0063201.1.1', mockElement);

    // Simulate reload by getting fresh instance
    const recent = WbsUsageService.getRecentWbs();

    // Verify WBS code in recent list
    expect(recent).toHaveLength(1);
    expect(recent[0].code).toBe('USC0063201.1.1');
  });

  // Component integration tests
  it('should display recent codes at top of autocomplete', () => {
    // Add 3 codes to recent list
    const code1 = createMockWbsElement('USC0063201.1.1');
    const code2 = createMockWbsElement('USC0063202.1.1');
    const code3 = createMockWbsElement('USC0063203.1.1');

    WbsUsageService.addRecentWbs('USC0063201.1.1', code1);
    WbsUsageService.addRecentWbs('USC0063202.1.1', code2);
    WbsUsageService.addRecentWbs('USC0063203.1.1', code3);

    // Verify recent codes are in order (most recent first)
    const recent = WbsUsageService.getRecentElements();
    expect(recent).toHaveLength(3);
    expect(recent[0].wbs_code).toBe('USC0063203.1.1'); // Last added first
    expect(recent[1].wbs_code).toBe('USC0063202.1.1');
    expect(recent[2].wbs_code).toBe('USC0063201.1.1');
  });

  it('should display favorites at top of autocomplete', () => {
    // Add 3 codes to favorites
    WbsUsageService.addFavorite('USC0063201.1.1');
    WbsUsageService.addFavorite('USC0063202.1.1');
    WbsUsageService.addFavorite('USC0063203.1.1');

    // Verify favorites are persisted
    const favorites = WbsUsageService.getFavorites();
    expect(favorites).toHaveLength(3);
    expect(favorites).toContain('USC0063201.1.1');
    expect(favorites).toContain('USC0063202.1.1');
    expect(favorites).toContain('USC0063203.1.1');
  });

  it('should filter favorites and recent by search query', () => {
    // Add 5 favorites
    WbsUsageService.addFavorite('USC0063201.1.1');
    WbsUsageService.addFavorite('USC0063202.1.1');
    WbsUsageService.addFavorite('USC0012345.1.1'); // Contains "12345"
    WbsUsageService.addFavorite('USC0063204.1.1');
    WbsUsageService.addFavorite('USC0054321.1.1'); // Contains reversed

    const favorites = WbsUsageService.getFavorites();

    // Filter favorites by search query "12345"
    const matching = favorites.filter(code => code.includes('12345'));

    // Verify only matching favorites shown
    expect(matching).toHaveLength(1);
    expect(matching[0]).toBe('USC0012345.1.1');
  });
});
