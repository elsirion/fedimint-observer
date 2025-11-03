import type { ReactNode } from 'react';

export type ButtonColorScheme = 'primary' | 'secondary' | 'success';

interface ButtonProps {
  colorScheme?: ButtonColorScheme;
  className?: string;
  onClick: () => void;
  disabled?: boolean;
  children: ReactNode;
}

const colorSchemes = {
  primary: {
    enabled: 'text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800',
    disabled: 'text-white bg-blue-400 dark:bg-blue-500 cursor-not-allowed',
  },
  secondary: {
    enabled: 'text-gray-900 focus:outline-none bg-gray-100 border border-gray-300 hover:bg-gray-200 hover:text-blue-700 focus:z-10 focus:ring-4 focus:ring-gray-100 dark:focus:ring-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-600 dark:hover:text-white dark:hover:bg-gray-700',
    disabled: 'text-gray-400 dark:text-gray-600 cursor-not-allowed',
  },
  success: {
    enabled: 'text-white bg-green-700 hover:bg-green-800 focus:ring-4 focus:ring-green-300 dark:bg-green-600 dark:hover:bg-green-700 focus:outline-none',
    disabled: 'text-white bg-green-400 dark:bg-green-500 cursor-not-allowed',
  },
};

export function Button({
  colorScheme = 'primary',
  className = '',
  onClick,
  disabled = false,
  children,
}: ButtonProps) {
  const scheme = colorSchemes[colorScheme];
  const colorClass = disabled ? scheme.disabled : scheme.enabled;
  const baseClass = 'whitespace-nowrap font-medium rounded-lg text-sm px-5 py-2.5';

  return (
    <button
      type="button"
      className={`${baseClass} ${colorClass} ${className}`}
      onClick={onClick}
      disabled={disabled}
    >
      {children}
    </button>
  );
}
