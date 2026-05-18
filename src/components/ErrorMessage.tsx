import type { ApiError } from '../lib/api';

interface ErrorMessageProps {
  error: ApiError | Error | string;
}

export default function ErrorMessage({ error }: ErrorMessageProps) {
  const apiError = typeof error === 'object' && 'code' in error ? (error as ApiError) : null;
  const message = typeof error === 'string' ? error : error.message;
  const prefix = apiError ? `[${apiError.code}] ` : '';
  const recoverable = apiError?.recoverable === false ? ' Restart may be required.' : '';

  return (
    <div className="dashboard__error" role="alert">
      {prefix}
      {message}
      {recoverable}
    </div>
  );
}
