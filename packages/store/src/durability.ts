/**
 * Storage durability, from the UI thread: StorageManager.persist() exists
 * only on window. A persisted origin is exempt from the browser's automatic
 * eviction under storage pressure. Without it, a local-first database can
 * be deleted with no warning.
 */

export interface StorageState {
  /** Whether the browser has granted persistent storage, or null if unsupported. */
  persisted: boolean | null;
  /** Bytes this origin uses, if reported. */
  usage: number | null;
  /** Bytes this origin may use, if reported. */
  quota: number | null;
}

/**
 * Asks the browser to make this origin's storage persistent. Chrome and Edge
 * decide silently from engagement (installed apps are granted), Firefox may
 * prompt, and Safari grants it based on its own heuristics. Safe to call on
 * every start.
 */
export async function requestPersistence(): Promise<boolean | null> {
  if (!navigator.storage?.persist) return null;
  if (await navigator.storage.persisted()) return true;
  return navigator.storage.persist();
}

export async function persisted(): Promise<boolean | null> {
  if (!navigator.storage?.persisted) return null;
  return navigator.storage.persisted();
}

export async function estimate(): Promise<Pick<StorageState, "usage" | "quota">> {
  if (!navigator.storage?.estimate) return { usage: null, quota: null };
  const { usage, quota } = await navigator.storage.estimate();
  return { usage: usage ?? null, quota: quota ?? null };
}

export async function storageState(): Promise<StorageState> {
  const [isPersisted, sizes] = await Promise.all([persisted(), estimate()]);
  return { persisted: isPersisted, ...sizes };
}
