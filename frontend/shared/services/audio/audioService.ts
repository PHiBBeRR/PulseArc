/**
 * Shared Audio Service
 * Manages a singleton AudioContext for UI sound feedback across the application.
 * Provides centralized lifecycle management and cleanup.
 */

type AudioServiceConfig = {
  frequency?: number;
  gain?: number;
  duration?: number;
};

class AudioService {
  private static instance: AudioService | null = null;
  private audioContext: AudioContext | null = null;
  private isInitialized = false;

  private constructor() {
    // Private constructor for singleton pattern
  }

  /**
   * Get singleton instance
   */
  static getInstance(): AudioService {
    if (!AudioService.instance) {
      AudioService.instance = new AudioService();
    }
    return AudioService.instance;
  }

  /**
   * Initialize audio context (lazy initialization)
   */
  private initialize(): void {
    if (this.isInitialized) return;

    try {
      const AudioContextConstructor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;

      this.audioContext = new AudioContextConstructor();
      this.isInitialized = true;
    } catch (error) {
      console.warn('Failed to initialize AudioContext:', error);
      this.isInitialized = false;
    }
  }

  /**
   * Play a click sound
   */
  playClick(config?: AudioServiceConfig): void {
    try {
      if (!this.isInitialized) {
        this.initialize();
      }

      if (!this.audioContext) {
        return; // Silently fail if context unavailable
      }

      const frequency = config?.frequency ?? 800;
      const gain = config?.gain ?? 0.1;
      const duration = config?.duration ?? 0.1;

      const oscillator = this.audioContext.createOscillator();
      const gainNode = this.audioContext.createGain();

      oscillator.connect(gainNode);
      gainNode.connect(this.audioContext.destination);

      oscillator.frequency.value = frequency;
      oscillator.type = 'sine';

      gainNode.gain.setValueAtTime(gain, this.audioContext.currentTime);
      gainNode.gain.exponentialRampToValueAtTime(0.01, this.audioContext.currentTime + duration);

      oscillator.start(this.audioContext.currentTime);
      oscillator.stop(this.audioContext.currentTime + duration);

      // Clean up nodes after sound finishes to prevent memory leak
      oscillator.onended = () => {
        try {
          gainNode.disconnect();
          oscillator.disconnect();
        } catch {
          // Nodes already disconnected, ignore
        }
      };
    } catch (error) {
      // Silently fail if audio playback fails
      console.debug('Audio playback failed:', error);
    }
  }

  /**
   * Get the current AudioContext (for advanced usage)
   */
  getContext(): AudioContext | null {
    if (!this.isInitialized) {
      this.initialize();
    }
    return this.audioContext;
  }

  /**
   * Cleanup audio context
   * Should be called when the application is shutting down
   */
  async cleanup(): Promise<void> {
    if (this.audioContext) {
      try {
        await this.audioContext.close();
      } catch (error) {
        console.warn('Failed to close AudioContext:', error);
      } finally {
        this.audioContext = null;
        this.isInitialized = false;
      }
    }
  }

  /**
   * Reset singleton (primarily for testing)
   */
  static reset(): void {
    if (AudioService.instance?.audioContext) {
      void AudioService.instance.cleanup();
    }
    AudioService.instance = null;
  }
}

// Export singleton instance
export const audioService = AudioService.getInstance();
export type { AudioServiceConfig };
