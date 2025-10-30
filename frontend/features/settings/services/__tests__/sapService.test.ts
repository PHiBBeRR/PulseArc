// FEATURE-020 Phase 2: SAP Service Tests

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { WbsElement, OutboxStatusSummary } from '@/shared/types/generated';

// Mock Tauri invoke (must be declared before imports that use it)
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { SapService } from '../sapService';
import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe('SapService', () => {
  beforeEach(() => {
    mockInvoke.mockClear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Authentication', () => {
    it('should start SAP OAuth login', async () => {
      const mockAuthUrl = 'https://auth0.example.com/authorize?...';
      mockInvoke.mockResolvedValueOnce(mockAuthUrl);

      const result = await SapService.startLogin();

      expect(mockInvoke).toHaveBeenCalledWith('sap_start_login');
      expect(result).toBe(mockAuthUrl);
    });

    it('should complete SAP OAuth login', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await SapService.completeLogin('auth_code_123', 'state_456');

      expect(mockInvoke).toHaveBeenCalledWith('sap_complete_login', {
        code: 'auth_code_123',
        state: 'state_456',
      });
    });

    it('should check authentication status', async () => {
      mockInvoke.mockResolvedValueOnce(true);

      const result = await SapService.isAuthenticated();

      expect(mockInvoke).toHaveBeenCalledWith('sap_is_authenticated');
      expect(result).toBe(true);
    });

    it('should return false when not authenticated', async () => {
      mockInvoke.mockResolvedValueOnce(false);

      const result = await SapService.isAuthenticated();

      expect(result).toBe(false);
    });

    it('should logout from SAP', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await SapService.logout();

      expect(mockInvoke).toHaveBeenCalledWith('sap_logout');
    });

    it('should get auth status with timestamp', async () => {
      mockInvoke.mockResolvedValueOnce(true);
      const beforeTimestamp = Date.now();

      const result = await SapService.getAuthStatus();

      const afterTimestamp = Date.now();
      expect(result.isAuthenticated).toBe(true);
      expect(result.lastChecked).toBeGreaterThanOrEqual(beforeTimestamp);
      expect(result.lastChecked).toBeLessThanOrEqual(afterTimestamp);
    });
  });

  describe('WBS Search', () => {
    // Use actual WBS codes from seed data (prisma/seed/wbs-mock-data.sql)
    const mockWbsElements: WbsElement[] = [
      {
        wbs_code: 'USC0063201.1.1',
        project_def: 'USC0063201',
        project_name: 'Project Astro - Tech Acquisition',
        description: 'Project Astro - Deals - M&A Tax',
        status: 'REL',
        cached_at: Date.now(),
        // FEATURE-029: Enriched opportunity metadata
        opportunity_id: null,
        deal_name: null,
        target_company_name: null,
        counterparty: null,
        industry: null,
        region: null,
        amount: null,
        stage_name: null,
        project_code: null,
      },
      {
        wbs_code: 'USC0063202.1.1',
        project_def: 'USC0063202',
        project_name: 'Project Beta - Pharma Merger',
        description: 'Project Beta - Deals - M&A Tax',
        status: 'REL',
        cached_at: Date.now(),
        // FEATURE-029: Enriched opportunity metadata
        opportunity_id: null,
        deal_name: null,
        target_company_name: null,
        counterparty: null,
        industry: null,
        region: null,
        amount: null,
        stage_name: null,
        project_code: null,
      },
    ];

    it('should search WBS codes', async () => {
      mockInvoke.mockResolvedValueOnce(mockWbsElements);

      const result = await SapService.searchWbs('Project');

      expect(mockInvoke).toHaveBeenCalledWith('sap_search_wbs', {
        query: 'Project',
      });
      expect(result).toEqual(mockWbsElements);
    });

    it('should trim search query', async () => {
      mockInvoke.mockResolvedValueOnce([mockWbsElements[0]]);

      await SapService.searchWbs('  Astro  ');

      expect(mockInvoke).toHaveBeenCalledWith('sap_search_wbs', {
        query: 'Astro',
      });
    });

    it('should return empty array for empty query', async () => {
      const result = await SapService.searchWbs('');

      expect(mockInvoke).not.toHaveBeenCalled();
      expect(result).toEqual([]);
    });

    it('should return empty array for whitespace-only query', async () => {
      const result = await SapService.searchWbs('   ');

      expect(mockInvoke).not.toHaveBeenCalled();
      expect(result).toEqual([]);
    });
  });

  describe('Outbox Management', () => {
    const mockOutboxSummary: OutboxStatusSummary = {
      pending_count: 3,
      sent_count: 10,
      failed_count: 1,
    };

    it('should get outbox status', async () => {
      mockInvoke.mockResolvedValueOnce(mockOutboxSummary);

      const result = await SapService.getOutboxStatus();

      expect(mockInvoke).toHaveBeenCalledWith('sap_get_outbox_status');
      expect(result).toEqual({
        pending: 3,
        sent: 10,
        failed: 1,
      });
    });

    it('should retry failed entries', async () => {
      mockInvoke.mockResolvedValueOnce(5);

      const result = await SapService.retryFailedEntries();

      expect(mockInvoke).toHaveBeenCalledWith('sap_retry_failed_entries');
      expect(result).toBe(5);
    });

    it('should start forwarder', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await SapService.startForwarder();

      expect(mockInvoke).toHaveBeenCalledWith('sap_start_forwarder');
    });

    it('should stop forwarder', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await SapService.stopForwarder();

      expect(mockInvoke).toHaveBeenCalledWith('sap_stop_forwarder');
    });
  });

  describe('Utility Functions', () => {
    it('should format WBS display with all fields', () => {
      const element: WbsElement = {
        wbs_code: 'USC0063201.1.1',
        project_def: 'USC0063201',
        project_name: 'Project Astro - Tech Acquisition',
        description: 'Project Astro - Deals - M&A Tax',
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
      };

      const result = SapService.formatWbsDisplay(element);

      expect(result).toBe('USC0063201.1.1 - Project Astro - Tech Acquisition - (Project Astro - Deals - M&A Tax)');
    });

    it('should format WBS display without project name', () => {
      const element: WbsElement = {
        wbs_code: 'USC0063201.1.1',
        project_def: 'USC0063201',
        project_name: null,
        description: 'Project Astro - Deals - M&A Tax',
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
      };

      const result = SapService.formatWbsDisplay(element);

      expect(result).toBe('USC0063201.1.1 - (Project Astro - Deals - M&A Tax)');
    });

    it('should format WBS display without description', () => {
      const element: WbsElement = {
        wbs_code: 'USC0063201.1.1',
        project_def: 'USC0063201',
        project_name: 'Project Astro - Tech Acquisition',
        description: null,
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
      };

      const result = SapService.formatWbsDisplay(element);

      expect(result).toBe('USC0063201.1.1 - Project Astro - Tech Acquisition');
    });

    it('should format WBS display with only code', () => {
      const element: WbsElement = {
        wbs_code: 'USC0063201.1.1',
        project_def: 'USC0063201',
        project_name: null,
        description: null,
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
      };

      const result = SapService.formatWbsDisplay(element);

      expect(result).toBe('USC0063201.1.1');
    });

    it('should validate correct WBS code format', () => {
      // Valid format: [ProjectCode].[Platform].[Team]
      expect(SapService.validateWbsCode('USC0063201.1.1')).toBe(true);
      expect(SapService.validateWbsCode('USC0063202.2.3')).toBe(true);
      expect(SapService.validateWbsCode('USC0063210.3.4')).toBe(true);
    });

    it('should reject invalid WBS code format', () => {
      expect(SapService.validateWbsCode('')).toBe(false);
      expect(SapService.validateWbsCode('USC0063201')).toBe(false); // Missing platform.team
      expect(SapService.validateWbsCode('USC0063201.1')).toBe(false); // Missing team
      expect(SapService.validateWbsCode('PROJ-001-WBS01')).toBe(false); // Old format
      expect(SapService.validateWbsCode('USC0063201.4.1')).toBe(false); // Invalid platform (max 3)
      expect(SapService.validateWbsCode('USC0063201.1.5')).toBe(false); // Invalid team (max 4)
      expect(SapService.validateWbsCode('123.1.1')).toBe(false); // Invalid project code format
    });
  });

  describe('Error Handling', () => {
    it('should handle auth errors gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Auth0 connection failed'));

      await expect(SapService.startLogin()).rejects.toThrow('Auth0 connection failed');
    });

    it('should handle search errors gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Database error'));

      await expect(SapService.searchWbs('test')).rejects.toThrow('Database error');
    });

    it('should handle outbox errors gracefully', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Failed to query outbox'));

      await expect(SapService.getOutboxStatus()).rejects.toThrow('Failed to query outbox');
    });
  });
});
