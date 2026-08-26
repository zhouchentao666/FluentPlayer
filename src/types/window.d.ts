export {}

declare global {
  interface Window {
    api: typeof import('../bridge/index')['api']
    electron: {
      ipcRenderer: {
        invoke(channel: string, ...args: any[]): Promise<any>
        send(channel: string, ...args: any[]): void
        on(
          channel: string,
          callback: (event: any, ...args: any[]) => void
        ): Promise<() => void>
        removeAllListeners(channel: string): Promise<void>
        removeListener(channel: string, handler: Function): void
      }
    }
  }
}
