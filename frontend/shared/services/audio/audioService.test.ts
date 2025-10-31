/**
 * Unit tests for AudioService
 *
 * Tests the audio feedback system that provides click sounds and other
 * audio cues for user interactions throughout the application.
 *
 * Test Coverage:
 * - Click Sounds: Generating UI feedback clicks using Web Audio API
 * - Audio Context: Initialization, cleanup, and state management
 * - Oscillator Management: Creating and controlling sound oscillators
 * - Gain Control: Volume management and fade effects
 * - Error Handling: Graceful degradation when audio is unavailable
 * - Resource Cleanup: Proper disposal of audio resources
 */

/* eslint-disable @typescript-eslint/no-explicit-any */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { audioService } from './audioService';

describe('AudioService', () => {
  let mockAudioContext: any;
  let mockOscillator: any;
  let mockGainNode: any;
  let oscillators: any[];
  let gainNodes: any[];
  let originalAudioContext: any;

  beforeEach(() => {
    oscillators = [];
    gainNodes = [];

    // Save original AudioContext if it exists
    originalAudioContext = (global as any).AudioContext;

    // Create mock AudioContext
    mockAudioContext = {
      close: vi.fn().mockResolvedValue(undefined),
      createOscillator: vi.fn(() => {
        const osc = {
          connect: vi.fn(),
          disconnect: vi.fn(),
          start: vi.fn(),
          stop: vi.fn(),
          frequency: { value: 0 },
          type: 'sine',
          onended: null,
        };
        oscillators.push(osc);
        mockOscillator = osc; // Keep reference to last created oscillator
        return osc;
      }),
      createGain: vi.fn(() => {
        const gain = {
          connect: vi.fn(),
          disconnect: vi.fn(),
          gain: {
            setValueAtTime: vi.fn(),
            exponentialRampToValueAtTime: vi.fn(),
          },
        };
        gainNodes.push(gain);
        mockGainNode = gain; // Keep reference to last created gain node
        return gain;
      }),
      destination: {},
      currentTime: 0,
    };

    // Mock global AudioContext constructor (Vi test 4.x requires proper constructor)
    global.AudioContext = vi.fn().mockImplementation(function (this: any) {
      return mockAudioContext;
    }) as any;
  });

  afterEach(async () => {
    // Cleanup singleton after each test (don't reset to avoid stale reference issues)
    await audioService.cleanup();

    // Restore original AudioContext
    if (originalAudioContext) {
      (global as any).AudioContext = originalAudioContext;
    }

    // Remove webkit fallback if it was added
    delete (global as any).webkitAudioContext;

    vi.clearAllMocks();

    // Clear the tracking arrays
    oscillators = [];
    gainNodes = [];
  });

  describe('Singleton Pattern', () => {
    it('should return the same instance on multiple getInstance calls', () => {
      const instance1 = (audioService as any).constructor.getInstance();
      const instance2 = (audioService as any).constructor.getInstance();
      expect(instance1).toBe(instance2);
    });

    it('should export a singleton instance by default', () => {
      expect(audioService).toBeDefined();
      expect(typeof audioService.playClick).toBe('function');
    });
  });

  describe('Lazy Initialization', () => {
    it('should not initialize AudioContext on import', () => {
      expect(global.AudioContext).not.toHaveBeenCalled();
    });

    it('should initialize AudioContext on first playClick call', () => {
      expect(global.AudioContext).not.toHaveBeenCalled();

      audioService.playClick();

      expect(global.AudioContext).toHaveBeenCalledTimes(1);
    });

    it('should not reinitialize AudioContext on subsequent playClick calls', () => {
      audioService.playClick();
      audioService.playClick();
      audioService.playClick();

      expect(global.AudioContext).toHaveBeenCalledTimes(1);
    });

    it('should initialize AudioContext when getContext is called', () => {
      expect(global.AudioContext).not.toHaveBeenCalled();

      const context = audioService.getContext();

      expect(global.AudioContext).toHaveBeenCalledTimes(1);
      expect(context).toBe(mockAudioContext);
    });
  });

  describe('AudioContext Reuse', () => {
    it('should reuse the same AudioContext for multiple playClick calls', () => {
      audioService.playClick();
      const firstContext = audioService.getContext();

      audioService.playClick();
      const secondContext = audioService.getContext();

      audioService.playClick();
      const thirdContext = audioService.getContext();

      expect(firstContext).toBe(secondContext);
      expect(secondContext).toBe(thirdContext);
      expect(global.AudioContext).toHaveBeenCalledTimes(1);
    });

    it('should use the same context for oscillator and gain node creation', () => {
      audioService.playClick();
      audioService.playClick();

      expect(mockAudioContext.createOscillator).toHaveBeenCalledTimes(2);
      expect(mockAudioContext.createGain).toHaveBeenCalledTimes(2);
    });
  });

  describe('playClick - Default Configuration', () => {
    it('should create oscillator and gain nodes', () => {
      audioService.playClick();

      expect(mockAudioContext.createOscillator).toHaveBeenCalled();
      expect(mockAudioContext.createGain).toHaveBeenCalled();
    });

    it('should connect oscillator to gain node', () => {
      audioService.playClick();

      expect(mockOscillator.connect).toHaveBeenCalledWith(mockGainNode);
    });

    it('should connect gain node to destination', () => {
      audioService.playClick();

      expect(mockGainNode.connect).toHaveBeenCalledWith(mockAudioContext.destination);
    });

    it('should set default frequency to 800Hz', () => {
      audioService.playClick();

      expect(mockOscillator.frequency.value).toBe(800);
    });

    it('should set oscillator type to sine', () => {
      audioService.playClick();

      expect(mockOscillator.type).toBe('sine');
    });

    it('should set default gain to 0.1', () => {
      audioService.playClick();

      expect(mockGainNode.gain.setValueAtTime).toHaveBeenCalledWith(0.1, 0);
    });

    it('should ramp gain down to 0.01 over default duration', () => {
      audioService.playClick();

      expect(mockGainNode.gain.exponentialRampToValueAtTime).toHaveBeenCalledWith(0.01, 0.1);
    });

    it('should start and stop oscillator', () => {
      audioService.playClick();

      expect(mockOscillator.start).toHaveBeenCalledWith(0);
      expect(mockOscillator.stop).toHaveBeenCalledWith(0.1);
    });
  });

  describe('playClick - Custom Configuration', () => {
    it('should use custom frequency', () => {
      audioService.playClick({ frequency: 1000 });

      expect(mockOscillator.frequency.value).toBe(1000);
    });

    it('should use custom gain', () => {
      audioService.playClick({ gain: 0.5 });

      expect(mockGainNode.gain.setValueAtTime).toHaveBeenCalledWith(0.5, 0);
    });

    it('should use custom duration', () => {
      audioService.playClick({ duration: 0.2 });

      expect(mockGainNode.gain.exponentialRampToValueAtTime).toHaveBeenCalledWith(0.01, 0.2);
      expect(mockOscillator.stop).toHaveBeenCalledWith(0.2);
    });

    it('should use all custom config values together', () => {
      audioService.playClick({ frequency: 1200, gain: 0.3, duration: 0.15 });

      expect(mockOscillator.frequency.value).toBe(1200);
      expect(mockGainNode.gain.setValueAtTime).toHaveBeenCalledWith(0.3, 0);
      expect(mockGainNode.gain.exponentialRampToValueAtTime).toHaveBeenCalledWith(0.01, 0.15);
      expect(mockOscillator.stop).toHaveBeenCalledWith(0.15);
    });

    it('should merge partial config with defaults', () => {
      audioService.playClick({ frequency: 900 });

      expect(mockOscillator.frequency.value).toBe(900);
      expect(mockGainNode.gain.setValueAtTime).toHaveBeenCalledWith(0.1, 0); // default gain
      expect(mockOscillator.stop).toHaveBeenCalledWith(0.1); // default duration
    });
  });

  describe('cleanup', () => {
    it('should close AudioContext when cleanup is called', async () => {
      audioService.playClick(); // Initialize

      await audioService.cleanup();

      expect(mockAudioContext.close).toHaveBeenCalledTimes(1);
    });

    it('should reset initialized state after cleanup', async () => {
      audioService.playClick(); // Initialize
      await audioService.cleanup();

      // Next playClick should create a new AudioContext
      audioService.playClick();

      expect(global.AudioContext).toHaveBeenCalledTimes(2);
    });

    it('should clear audioContext reference after cleanup', async () => {
      audioService.playClick(); // Initialize
      await audioService.cleanup();

      const context = audioService.getContext();

      // After cleanup, getContext should reinitialize
      expect(global.AudioContext).toHaveBeenCalledTimes(2);
      expect(context).toBe(mockAudioContext);
    });

    it('should not throw if cleanup is called without initialization', async () => {
      await expect(audioService.cleanup()).resolves.toBeUndefined();
    });

    it('should handle close errors gracefully', async () => {
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      mockAudioContext.close = vi.fn().mockRejectedValue(new Error('Close failed'));

      audioService.playClick(); // Initialize

      await audioService.cleanup();

      expect(consoleWarnSpy).toHaveBeenCalledWith(
        'Failed to close AudioContext:',
        expect.any(Error)
      );

      consoleWarnSpy.mockRestore();
    });
  });

  describe('Error Handling', () => {
    it('should handle AudioContext constructor errors gracefully', () => {
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      global.AudioContext = vi.fn(() => {
        throw new Error('AudioContext not supported');
      }) as any;

      // Should not throw
      expect(() => audioService.playClick()).not.toThrow();

      expect(consoleWarnSpy).toHaveBeenCalledWith(
        'Failed to initialize AudioContext:',
        expect.any(Error)
      );

      consoleWarnSpy.mockRestore();
    });

    it('should silently fail playClick if AudioContext unavailable', () => {
      const consoleDebugSpy = vi.spyOn(console, 'debug').mockImplementation(() => {});
      global.AudioContext = vi.fn(() => {
        throw new Error('AudioContext not supported');
      }) as any;

      audioService.playClick();

      // Should not create oscillator
      expect(mockAudioContext.createOscillator).not.toHaveBeenCalled();

      consoleDebugSpy.mockRestore();
    });

    it('should handle playback errors gracefully', () => {
      const consoleDebugSpy = vi.spyOn(console, 'debug').mockImplementation(() => {});
      mockAudioContext.createOscillator = vi.fn(() => {
        throw new Error('Oscillator creation failed');
      });

      // Should not throw
      expect(() => audioService.playClick()).not.toThrow();

      expect(consoleDebugSpy).toHaveBeenCalledWith('Audio playback failed:', expect.any(Error));

      consoleDebugSpy.mockRestore();
    });

    it('should return null from getContext if initialization fails', async () => {
      // First cleanup to reset state
      await audioService.cleanup();

      // Then mock AudioContext to fail
      global.AudioContext = vi.fn(() => {
        throw new Error('AudioContext not supported');
      }) as any;

      const context = audioService.getContext();

      expect(context).toBeNull();
    });
  });

  describe('reset (Test Utility)', () => {
    it('should reset singleton instance', () => {
      const instance1 = (audioService as any).constructor.getInstance();

      (audioService as any).constructor.reset();

      const instance2 = (audioService as any).constructor.getInstance();

      // After reset, getInstance should create a new instance
      expect(instance1).not.toBe(instance2);
    });

    it('should allow new initialization after reset', async () => {
      audioService.playClick(); // Initialize

      // Wait to ensure initialization completes
      await new Promise((resolve) => setTimeout(resolve, 0));

      (audioService as any).constructor.reset();

      // Wait for reset to complete
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(global.AudioContext).toHaveBeenCalledTimes(1);

      // Create new instance and initialize
      const newService = (audioService as any).constructor.getInstance();
      newService.playClick();

      expect(global.AudioContext).toHaveBeenCalledTimes(2);
    });
  });

  describe('Browser Compatibility', () => {
    it('should support webkitAudioContext fallback', async () => {
      // Cleanup first to reset state
      await audioService.cleanup();

      // Remove AudioContext and add webkitAudioContext
      delete (global as any).AudioContext;
      (global as any).webkitAudioContext = vi.fn().mockImplementation(function (this: any) {
        return mockAudioContext;
      });

      // Reset singleton to pick up new context
      (audioService as any).constructor.reset();
      const newService = (audioService as any).constructor.getInstance();

      // playClick should use webkitAudioContext
      newService.playClick();

      expect((global as any).webkitAudioContext).toHaveBeenCalled();
      expect(mockAudioContext.createOscillator).toHaveBeenCalled();
    });
  });

  describe('Memory Leak Prevention', () => {
    it('should disconnect audio nodes after sound finishes playing', () => {
      audioService.playClick();

      // Verify oscillator was created
      expect(oscillators.length).toBe(1);
      expect(gainNodes.length).toBe(1);

      // Get the oscillator that was created
      const osc = oscillators[0];
      const gain = gainNodes[0];

      // Verify onended handler was set
      expect(osc.onended).toBeDefined();
      expect(typeof osc.onended).toBe('function');

      // Simulate the oscillator ending
      if (osc.onended) {
        osc.onended(new Event('ended'));
      }

      // Verify nodes were disconnected to prevent memory leak
      expect(gain.disconnect).toHaveBeenCalledTimes(1);
      expect(osc.disconnect).toHaveBeenCalledTimes(1);
    });

    it('should handle disconnect errors gracefully', () => {
      audioService.playClick();

      // Verify nodes were created
      expect(oscillators.length).toBeGreaterThan(0);
      expect(gainNodes.length).toBeGreaterThan(0);

      const osc = oscillators[0];
      const gain = gainNodes[0];

      // Make disconnect throw an error
      gain.disconnect.mockImplementation(() => {
        throw new Error('Already disconnected');
      });

      // Should not throw when onended is called even if disconnect fails
      expect(() => {
        if (osc.onended) {
          osc.onended(new Event('ended'));
        }
      }).not.toThrow();

      expect(gain.disconnect).toHaveBeenCalled();
    });

    it('should not accumulate nodes after multiple playClick calls', () => {
      // Play 10 sounds
      for (let i = 0; i < 10; i++) {
        audioService.playClick();
      }

      // Should have created 10 oscillators
      expect(mockAudioContext.createOscillator).toHaveBeenCalledTimes(10);
      expect(mockAudioContext.createGain).toHaveBeenCalledTimes(10);
      expect(oscillators.length).toBe(10);
      expect(gainNodes.length).toBe(10);

      // All oscillators should have onended handlers
      oscillators.forEach((osc) => {
        expect(osc.onended).toBeDefined();
      });

      // Simulate all sounds ending
      oscillators.forEach((osc) => {
        if (osc.onended) {
          osc.onended(new Event('ended'));
        }
      });

      // All nodes should be disconnected (1 per oscillator)
      gainNodes.forEach((gain) => {
        expect(gain.disconnect).toHaveBeenCalledTimes(1);
      });
      oscillators.forEach((osc) => {
        expect(osc.disconnect).toHaveBeenCalledTimes(1);
      });
    });

    it('should clean up nodes even with custom config', () => {
      audioService.playClick({ frequency: 1000, gain: 0.5, duration: 0.2 });

      // Verify nodes were created
      expect(oscillators.length).toBeGreaterThan(0);
      expect(gainNodes.length).toBeGreaterThan(0);

      const osc = oscillators[0];
      const gain = gainNodes[0];

      expect(osc.onended).toBeDefined();

      // Simulate sound ending
      if (osc.onended) {
        osc.onended(new Event('ended'));
      }

      // Verify cleanup happened
      expect(gain.disconnect).toHaveBeenCalledTimes(1);
      expect(osc.disconnect).toHaveBeenCalledTimes(1);
    });
  });
});
