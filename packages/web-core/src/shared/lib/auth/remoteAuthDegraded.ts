// TODO(local-first): remote auth degradation is dead in single-user mode.

export function getRemoteAuthDegradedMessage(
  _state: unknown,
  _t: (key: string) => string
): string {
  return '';
}
