// Haptic feedback utility
export const haptic = {
  light: () => {
    if ('vibrate' in navigator) {
      navigator.vibrate(10);
    }
  },
  medium: () => {
    if ('vibrate' in navigator) {
      navigator.vibrate(20);
    }
  },
  heavy: () => {
    if ('vibrate' in navigator) {
      navigator.vibrate(30);
    }
  },
  success: () => {
    if ('vibrate' in navigator) {
      navigator.vibrate([10, 50, 10]);
    }
  },
  error: () => {
    if ('vibrate' in navigator) {
      navigator.vibrate([20, 30, 20, 30, 20]);
    }
  },
};

// Confetti celebration utility
export const celebrateWithConfetti = (options?: {
  particleCount?: number;
  spread?: number;
  origin?: { x?: number; y?: number };
}) => {
  // Dynamic import for canvas-confetti
  void import('canvas-confetti').then((confetti) => {
    const defaults = {
      particleCount: 100,
      spread: 70,
      origin: { y: 0.6 },
    };

    void confetti.default({
      ...defaults,
      ...options,
    });
  });
};

// Milestone confetti - more dramatic
export const celebrateMilestone = () => {
  void import('canvas-confetti').then((confetti) => {
    const count = 200;
    const defaults = {
      origin: { y: 0.7 },
    };

    function fire(particleRatio: number, opts: Record<string, unknown>) {
      void confetti.default({
        ...defaults,
        ...opts,
        particleCount: Math.floor(count * particleRatio),
      });
    }

    fire(0.25, {
      spread: 26,
      startVelocity: 55,
    });
    fire(0.2, {
      spread: 60,
    });
    fire(0.35, {
      spread: 100,
      decay: 0.91,
      scalar: 0.8,
    });
    fire(0.1, {
      spread: 120,
      startVelocity: 25,
      decay: 0.92,
      scalar: 1.2,
    });
    fire(0.1, {
      spread: 120,
      startVelocity: 45,
    });
  });
};

// Quick confetti burst
export const quickConfetti = () => {
  void import('canvas-confetti').then((confetti) => {
    void confetti.default({
      particleCount: 50,
      spread: 50,
      origin: { y: 0.5 },
    });
  });
};
