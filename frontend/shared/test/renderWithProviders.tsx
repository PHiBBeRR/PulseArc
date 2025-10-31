/**
 * Test utility for rendering components with required providers
 * Automatically wraps components with ThemeProvider for testing
 */

import { ThemeProvider } from '@/shared/components/layout/ThemeProvider';
import { render, type RenderOptions } from '@testing-library/react';
import type { ReactElement } from 'react';

/**
 * Custom render function that wraps components with ThemeProvider
 * Use this instead of plain render() from @testing-library/react
 *
 * @example
 * import { renderWithProviders as render } from '@/shared/test/renderWithProviders';
 *
 * render(<MyComponent />);
 */
export function renderWithProviders(ui: ReactElement, options?: Omit<RenderOptions, 'wrapper'>) {
  return render(ui, {
    wrapper: ({ children }) => <ThemeProvider>{children}</ThemeProvider>,
    ...options,
  });
}

// Re-export everything from React Testing Library for convenience
export * from '@testing-library/react';
export { renderWithProviders as render };
