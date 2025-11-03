import type { ReactNode } from 'react';

export type BadgeLevel = 'info' | 'warning' | 'error' | 'success';

interface BadgeProps {
  level: BadgeLevel;
  tooltip?: string;
  children: ReactNode;
}

export function Badge({ level, tooltip, children }: BadgeProps) {
  const styles = {
    info: 'bg-indigo-600 text-white text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-indigo-500',
    warning: 'bg-yellow-100 text-yellow-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-yellow-900 dark:text-yellow-300',
    error: 'bg-red-100 text-red-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-red-900 dark:text-red-300',
    success: 'bg-green-100 text-green-800 text-xs font-medium me-2 px-2.5 py-0.5 rounded dark:bg-green-900 dark:text-green-300',
  };

  if (tooltip) {
    return (
      <span className={styles[level]}>
        <abbr title={tooltip}>{children}</abbr>
      </span>
    );
  }

  return <span className={styles[level]}>{children}</span>;
}
