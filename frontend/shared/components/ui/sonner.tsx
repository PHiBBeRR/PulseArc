'use client';

import { useTheme } from 'next-themes';
import { Toaster as Sonner, ToasterProps } from 'sonner';

const Toaster = ({ ...props }: ToasterProps) => {
  const { theme = 'system' } = useTheme();

  return (
    <Sonner
      theme={theme as ToasterProps['theme']}
      className="toaster group"
      toastOptions={{
        classNames: {
          toast:
            'backdrop-blur-[60px] bg-white/80 dark:bg-black/80 border-2 border-white/20 dark:border-white/10 shadow-[0_8px_32px_0_rgba(0,0,0,0.2)]',
          title: 'text-gray-900 dark:text-gray-100',
          description: 'text-gray-600 dark:text-gray-400',
          actionButton:
            'backdrop-blur-xl bg-blue-500/20 hover:bg-blue-500/30 dark:bg-blue-400/20 dark:hover:bg-blue-400/30 text-blue-700 dark:text-blue-300 border border-blue-500/30 dark:border-blue-400/30',
          cancelButton:
            'backdrop-blur-xl bg-white/20 hover:bg-white/30 dark:bg-white/10 dark:hover:bg-white/15 text-gray-900 dark:text-white border border-white/30 dark:border-white/20',
          closeButton:
            'backdrop-blur-xl bg-white/20 hover:bg-white/30 dark:bg-white/10 dark:hover:bg-white/15 text-gray-900 dark:text-white border border-white/30 dark:border-white/20',
        },
      }}
      style={
        {
          '--normal-bg': 'rgba(255, 255, 255, 0.8)',
          '--normal-text': 'var(--popover-foreground)',
          '--normal-border': 'rgba(255, 255, 255, 0.2)',
        } as React.CSSProperties
      }
      {...props}
    />
  );
};

export { Toaster };
