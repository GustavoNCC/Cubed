import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

type Handler = () => void;

interface UseAppEventsOptions {
  onServerStarted?: Handler;
  onServerStopped?: Handler;
  onServerCrashed?: Handler;
  onBackupCreated?: Handler;
  onResourceUpdated?: Handler;
  onTailscaleUpdated?: Handler;
}

export function useAppEvents(opts: UseAppEventsOptions) {
  useEffect(() => {
    const subs: Array<Promise<() => void>> = [];

    if (opts.onServerStarted)
      subs.push(listen("cubed://server.started", opts.onServerStarted));
    if (opts.onServerStopped)
      subs.push(listen("cubed://server.stopped", opts.onServerStopped));
    if (opts.onServerCrashed)
      subs.push(listen("cubed://server.crashed", opts.onServerCrashed));
    if (opts.onBackupCreated)
      subs.push(listen("cubed://backup.created", opts.onBackupCreated));
    if (opts.onResourceUpdated)
      subs.push(listen("cubed://resource.updated", opts.onResourceUpdated));
    if (opts.onTailscaleUpdated)
      subs.push(listen("cubed://tailscale.updated", opts.onTailscaleUpdated));

    return () => {
      subs.forEach(p => p.then(unlisten => unlisten()));
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
