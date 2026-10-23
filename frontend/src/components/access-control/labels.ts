import { AccessEventStatus } from "@/common/types";

/** i18n keys for event statuses, shared by the member and door pages. */
export const statusKey: Record<AccessEventStatus, string> = {
  not_started: "access.status_not_started",
  running: "access.status_running",
  paused: "access.status_paused",
  ended: "access.status_ended",
};

/** English labels for the admin pages, which aren't translated. */
export const statusLabel: Record<AccessEventStatus, string> = {
  not_started: "Not started",
  running: "Running",
  paused: "On a break",
  ended: "Ended",
};

/** A `YYYY-MM-DD` date as the browser's local date. */
export const localDate = (isoDate: string) =>
  new Date(`${isoDate}T00:00:00`).toLocaleDateString();

export const localTime = (isoDateTime: string) =>
  new Date(isoDateTime).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
