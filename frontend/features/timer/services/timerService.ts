// Timer business logic service

import { formatTime } from '@/shared/utils/timeFormat';

export const timerService = {
  /**
   * Format elapsed seconds into HH:MM:SS format
   */
  formatTime: (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  },

  /**
   * Format current time for display (uses user's time format preference)
   */
  formatCurrentTime: (date: Date = new Date()): string => {
    return formatTime(date);
  },

  /**
   * Get greeting based on time of day with rotating variations
   */
  getGreeting: (date: Date = new Date()): string => {
    const hour = date.getHours();

    // Greeting pools for different times of day
    const morningGreetings = [
      'Good morning',
      'Rise and shine',
      'Morning',
      'Fresh start ahead',
      'Time to shine',
      'Hey there',
      'Welcome back',
      'Good to see you',
      "Let's get started",
      'New day, new wins',
    ];

    const afternoonGreetings = [
      'Good afternoon',
      'Afternoon',
      'Keep it going',
      "You're on a roll",
      'Halfway there',
      'Still crushing it',
      'Looking good',
      'Nice progress',
      'Staying strong',
      'Welcome back',
    ];

    const eveningGreetings = [
      'Good evening',
      'Evening',
      'Nice work today',
      'Almost there',
      'Final push',
      'Finishing strong',
      'Still going',
      'Looking good',
      'Welcome back',
      'Great progress',
    ];

    const lateNightGreetings = [
      'Night owl mode',
      'Late night hustle',
      'Still at it',
      'Impressive dedication',
      'Welcome back',
      'Still going strong',
      'Nice commitment',
      'Late but great',
      'Dedication mode',
      'After hours excellence',
    ];

    // Select greeting pool based on time of day
    let greetingPool: string[];
    if (hour < 12) {
      greetingPool = morningGreetings;
    } else if (hour < 18) {
      greetingPool = afternoonGreetings;
    } else if (hour < 22) {
      greetingPool = eveningGreetings;
    } else {
      greetingPool = lateNightGreetings;
    }

    // Use date to create a stable but varying index (changes each hour)
    const dayMinutes = hour * 60 + date.getMinutes();
    const index = Math.floor(dayMinutes / 60) % greetingPool.length;

    return greetingPool[index] ?? 'Hello';
  },

  /**
   * Calculate idle time in minutes
   */
  calculateIdleMinutes: (lastActivityTime: number): number => {
    const now = Date.now();
    const idleMs = now - lastActivityTime;
    return Math.floor(idleMs / 60000);
  },

  /**
   * Check if user has been idle for threshold
   */
  checkIdleThreshold: (lastActivityTime: number, thresholdMinutes: number = 5): boolean => {
    const idleMinutes = timerService.calculateIdleMinutes(lastActivityTime);
    return idleMinutes >= thresholdMinutes;
  },

  /**
   * Detect if milestone reached (1h, 2h, 3h, etc.)
   */
  isMilestone: (elapsed: number): boolean => {
    return elapsed > 0 && elapsed % 3600 === 0;
  },

  /**
   * Get milestone hours
   */
  getMilestoneHours: (elapsed: number): number => {
    return Math.floor(elapsed / 3600);
  },
};
