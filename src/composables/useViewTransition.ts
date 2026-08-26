export const supportsViewTransition =
  typeof document !== 'undefined' && 'startViewTransition' in document

/**
 * Wrap a DOM state change in a View Transition.
 * Falls back to running the callback directly if the API is unavailable.
 */
export function withViewTransition(
  callback: () => Promise<void> | void,
): Promise<void> | void {
  if (!supportsViewTransition) {
    return callback()
  }
  const transition = document.startViewTransition(callback)
  // A duplicate shared-element name aborts capture before "finished" settles.
  // Attach handlers to every lifecycle promise so browser-generated rejections
  // do not become unhandled promise rejections; callback errors still reject here.
  transition.ready.catch(() => {})
  transition.finished.catch(() => {})
  return transition.updateCallbackDone
}
