// Shared IPC services
export {
  CompactModeConfig,
  NormalModeConfig,
  SystemTrayMenuItems,
  TauriEvents,
  ipcClient,
} from './ipcClient';

// Alias for backwards compatibility
export { ipcClient as TauriAPI } from './ipcClient';
