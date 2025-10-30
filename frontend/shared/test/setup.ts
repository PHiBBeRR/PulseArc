import { afterEach, vi } from 'vitest';
import { cleanup } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';

// Cleanup after each test
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  vi.clearAllTimers();
  vi.unstubAllGlobals();
});

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  transformCallback: vi.fn((callback) => callback),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
  emit: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    setSize: vi.fn(),
    hide: vi.fn(),
    show: vi.fn(),
    setFocus: vi.fn(),
  })),
  LogicalSize: vi.fn().mockImplementation(function (
    this: { width: number; height: number },
    width: number,
    height: number
  ) {
    this.width = width;
    this.height = height;
    return this;
  }),
}));

// Mock haptic feedback
vi.mock('@/shared/utils', () => ({
  haptic: {
    light: vi.fn(),
    medium: vi.fn(),
    heavy: vi.fn(),
  },
}));

// Mock ResizeObserver for Radix UI components
global.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
};

// Mock PointerEvent for Radix UI Select component
// Fix for "target.hasPointerCapture is not a function" error
if (!globalThis.PointerEvent) {
  class PointerEvent extends MouseEvent {
    height?: number;
    isPrimary?: boolean;
    pointerId?: number;
    pointerType?: string;
    pressure?: number;
    tangentialPressure?: number;
    tiltX?: number;
    tiltY?: number;
    twist?: number;
    width?: number;

    constructor(type: string, params: PointerEventInit = {}) {
      super(type, params);
      this.pointerId = params.pointerId ?? 0;
      this.width = params.width ?? 0;
      this.height = params.height ?? 0;
      this.pressure = params.pressure ?? 0;
      this.tangentialPressure = params.tangentialPressure ?? 0;
      this.tiltX = params.tiltX ?? 0;
      this.tiltY = params.tiltY ?? 0;
      this.twist = params.twist ?? 0;
      this.pointerType = params.pointerType ?? '';
      this.isPrimary = params.isPrimary ?? false;
    }
  }
  globalThis.PointerEvent = PointerEvent as never;
}

// Mock hasPointerCapture and pointer capture methods for JSDOM
if (typeof Element !== 'undefined') {
  Element.prototype.hasPointerCapture = Element.prototype.hasPointerCapture || function() {
    return false;
  };
  Element.prototype.setPointerCapture = Element.prototype.setPointerCapture || function() {};
  Element.prototype.releasePointerCapture = Element.prototype.releasePointerCapture || function() {};

  // Mock scrollIntoView for Radix UI Select
  Element.prototype.scrollIntoView = Element.prototype.scrollIntoView || function() {};
}
