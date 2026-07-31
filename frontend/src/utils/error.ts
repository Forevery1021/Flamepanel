export function getErrorMessage(e: unknown, fallback: string): string {
  const resp = (e as { response?: { data?: { message?: string } } }).response
  return resp?.data?.message || (e instanceof Error ? e.message : fallback)
}
