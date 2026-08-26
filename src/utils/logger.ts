type LogFn = (...args: unknown[]) => void

interface Logger {
  debug: LogFn
  info: LogFn
  warn: LogFn
  error: LogFn
}

function createLogger(namespace: string): Logger {
  const tag = `[${namespace}]`
  const isDev = import.meta.env.DEV

  return {
    debug: isDev ? (...args: unknown[]) => console.debug(tag, ...args) : () => {},
    info: (...args: unknown[]) => console.info(tag, ...args),
    warn: (...args: unknown[]) => console.warn(tag, ...args),
    error: (...args: unknown[]) => console.error(tag, ...args),
  }
}

export default createLogger
