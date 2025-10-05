export type AlertLevel = 'info' | 'warning' | 'error' | 'success';

interface AlertProps {
  title?: string;
  message: string;
  level: AlertLevel;
  className?: string;
}

export function Alert({ title, message, level, className = '' }: AlertProps) {
  const styles = {
    info: 'p-4 mb-4 text-sm text-blue-800 rounded-lg bg-blue-50 dark:bg-gray-800 dark:text-blue-400',
    warning: 'p-4 mb-4 text-sm text-yellow-800 rounded-lg bg-yellow-50 dark:bg-gray-800 dark:text-yellow-300',
    error: 'p-4 mb-4 text-sm text-red-800 rounded-lg bg-red-50 dark:bg-gray-800 dark:text-red-400',
    success: 'p-4 mb-4 text-sm text-green-800 rounded-lg bg-green-50 dark:bg-gray-800 dark:text-green-400',
  };

  const defaultTitles = {
    info: 'Info: ',
    warning: 'Warning: ',
    error: 'Error: ',
    success: 'Success: ',
  };

  const displayTitle = title || defaultTitles[level];

  return (
    <div className={`${styles[level]} ${className}`} role="alert">
      <span className="font-bold">{displayTitle}</span>
      {message}
    </div>
  );
}
