import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { idleDetectionService } from './idleDetectionService';

describe('idleDetectionService', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('calculateIdleMinutes', () => {
    it('should return 0 for current time', () => {
      const now = Date.now();
      expect(idleDetectionService.calculateIdleMinutes(now)).toBe(0);
    });

    it('should calculate 5 minutes correctly', () => {
      const fiveMinutesAgo = Date.now() - (5 * 60 * 1000);
      expect(idleDetectionService.calculateIdleMinutes(fiveMinutesAgo)).toBe(5);
    });

    it('should floor partial minutes', () => {
      const fourPointNineMinutes = Date.now() - (4 * 60 * 1000 + 59 * 1000);
      expect(idleDetectionService.calculateIdleMinutes(fourPointNineMinutes)).toBe(4);
    });

    it('should handle zero idle time', () => {
      const now = Date.now();
      expect(idleDetectionService.calculateIdleMinutes(now)).toBe(0);
    });

    it('should calculate large idle times correctly', () => {
      const twoHours = Date.now() - (120 * 60 * 1000);
      expect(idleDetectionService.calculateIdleMinutes(twoHours)).toBe(120);
    });

    it('should handle millisecond precision', () => {
      const oneMinute = Date.now() - 60000;
      expect(idleDetectionService.calculateIdleMinutes(oneMinute)).toBe(1);
    });
  });

  describe('isIdleThresholdExceeded', () => {
    it('should return false when below threshold', () => {
      const twoMinutesAgo = Date.now() - (2 * 60 * 1000);
      expect(idleDetectionService.isIdleThresholdExceeded(twoMinutesAgo, 5)).toBe(false);
    });

    it('should return true when at threshold', () => {
      const fiveMinutesAgo = Date.now() - (5 * 60 * 1000);
      expect(idleDetectionService.isIdleThresholdExceeded(fiveMinutesAgo, 5)).toBe(true);
    });

    it('should return true when exceeding threshold', () => {
      const tenMinutesAgo = Date.now() - (10 * 60 * 1000);
      expect(idleDetectionService.isIdleThresholdExceeded(tenMinutesAgo, 5)).toBe(true);
    });

    it('should use default threshold of 5 minutes when not specified', () => {
      const sixMinutesAgo = Date.now() - (6 * 60 * 1000);
      expect(idleDetectionService.isIdleThresholdExceeded(sixMinutesAgo)).toBe(true);
    });

    it('should return false when just below threshold', () => {
      const fourPointNineMinutes = Date.now() - (4 * 60 * 1000 + 59 * 1000);
      expect(idleDetectionService.isIdleThresholdExceeded(fourPointNineMinutes, 5)).toBe(false);
    });

    it('should handle different thresholds', () => {
      const fifteenMinutesAgo = Date.now() - (15 * 60 * 1000);
      expect(idleDetectionService.isIdleThresholdExceeded(fifteenMinutesAgo, 10)).toBe(true);
      expect(idleDetectionService.isIdleThresholdExceeded(fifteenMinutesAgo, 20)).toBe(false);
    });
  });

  describe('formatIdleDuration', () => {
    it('should format minutes only when < 60', () => {
      expect(idleDetectionService.formatIdleDuration(45)).toBe('45m');
    });

    it('should format hours only when minutes is 0', () => {
      expect(idleDetectionService.formatIdleDuration(120)).toBe('2h');
    });

    it('should format hours and minutes', () => {
      expect(idleDetectionService.formatIdleDuration(125)).toBe('2h 5m');
    });

    it('should handle edge cases', () => {
      expect(idleDetectionService.formatIdleDuration(0)).toBe('0m');
      expect(idleDetectionService.formatIdleDuration(1)).toBe('1m');
      expect(idleDetectionService.formatIdleDuration(60)).toBe('1h');
    });

    it('should handle single hour with minutes', () => {
      expect(idleDetectionService.formatIdleDuration(75)).toBe('1h 15m');
    });

    it('should handle large durations', () => {
      expect(idleDetectionService.formatIdleDuration(240)).toBe('4h');
      expect(idleDetectionService.formatIdleDuration(245)).toBe('4h 5m');
    });

    it('should handle 59 minutes correctly', () => {
      expect(idleDetectionService.formatIdleDuration(59)).toBe('59m');
    });

    it('should handle 61 minutes correctly', () => {
      expect(idleDetectionService.formatIdleDuration(61)).toBe('1h 1m');
    });
  });

  describe('getDefaultConfig', () => {
    it('should return default configuration', () => {
      const config = idleDetectionService.getDefaultConfig();
      expect(config).toEqual({
        idleThresholdMinutes: 5,
        checkIntervalSeconds: 30,
        enableIdleDetection: true,
      });
    });

    it('should return a new object each time (not reference)', () => {
      const config1 = idleDetectionService.getDefaultConfig();
      const config2 = idleDetectionService.getDefaultConfig();
      expect(config1).not.toBe(config2);
      expect(config1).toEqual(config2);
    });
  });

  describe('calculateTimeToDiscard', () => {
    it('should convert minutes to seconds', () => {
      expect(idleDetectionService.calculateTimeToDiscard(5)).toBe(300);
      expect(idleDetectionService.calculateTimeToDiscard(10)).toBe(600);
      expect(idleDetectionService.calculateTimeToDiscard(0)).toBe(0);
    });

    it('should handle large values', () => {
      expect(idleDetectionService.calculateTimeToDiscard(120)).toBe(7200);
    });

    it('should handle single minute', () => {
      expect(idleDetectionService.calculateTimeToDiscard(1)).toBe(60);
    });
  });

  describe('getIdleMessage', () => {
    it('should format message with duration', () => {
      expect(idleDetectionService.getIdleMessage(5)).toBe(
        "You've been idle for 5m. Would you like to keep or discard this time?"
      );
    });

    it('should format message with hours and minutes', () => {
      expect(idleDetectionService.getIdleMessage(125)).toBe(
        "You've been idle for 2h 5m. Would you like to keep or discard this time?"
      );
    });

    it('should format message with hours only', () => {
      expect(idleDetectionService.getIdleMessage(60)).toBe(
        "You've been idle for 1h. Would you like to keep or discard this time?"
      );
    });

    it('should handle zero minutes', () => {
      expect(idleDetectionService.getIdleMessage(0)).toBe(
        "You've been idle for 0m. Would you like to keep or discard this time?"
      );
    });
  });

  describe('getIdleSeverity', () => {
    it('should return low for < 15 minutes', () => {
      expect(idleDetectionService.getIdleSeverity(10)).toBe('low');
      expect(idleDetectionService.getIdleSeverity(14)).toBe('low');
    });

    it('should return medium for 15-29 minutes', () => {
      expect(idleDetectionService.getIdleSeverity(15)).toBe('medium');
      expect(idleDetectionService.getIdleSeverity(29)).toBe('medium');
    });

    it('should return high for >= 30 minutes', () => {
      expect(idleDetectionService.getIdleSeverity(30)).toBe('high');
      expect(idleDetectionService.getIdleSeverity(120)).toBe('high');
    });

    it('should handle boundary values', () => {
      expect(idleDetectionService.getIdleSeverity(0)).toBe('low');
      expect(idleDetectionService.getIdleSeverity(14)).toBe('low');
      expect(idleDetectionService.getIdleSeverity(15)).toBe('medium');
      expect(idleDetectionService.getIdleSeverity(29)).toBe('medium');
      expect(idleDetectionService.getIdleSeverity(30)).toBe('high');
    });
  });
});
