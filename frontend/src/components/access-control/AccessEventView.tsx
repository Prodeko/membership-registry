import { useState } from "react";
import { Link, useParams } from "react-router";
import { ArrowLeft, Camera, Download, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { AccessEntry, AccessEventStatus } from "@/common/types";
import {
  useChangeAccessEventStatus,
  useDeleteAccessEntry,
  useExportAccessEvent,
  useGetAccessEvent,
  useTakeAccessSnapshot,
} from "@/lib/api";
import { cn, describeError } from "@/lib/utils";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Input } from "../ui/input";
import { localDate, localTime, statusLabel } from "./labels";

/** Status changes offered from each status, as [target, button label]. */
const transitions: Record<AccessEventStatus, [AccessEventStatus, string][]> = {
  not_started: [["running", "Start event"]],
  running: [
    ["paused", "Start a break"],
    ["ended", "End event"],
  ],
  paused: [
    ["running", "Resume event"],
    ["ended", "End event"],
  ],
  ended: [],
};

const methodLabel = { qr: "QR", manual: "Manual", auto: "Automatic" };

/// One event for door staff and admins: status, who is inside, snapshots
/// for votes, the full log and the export.
export default function AccessEventView() {
  const { id = "" } = useParams<{ id: string }>();
  const { data, isLoading, error } = useGetAccessEvent(id);
  const changeStatus = useChangeAccessEventStatus();
  const deleteEntry = useDeleteAccessEntry(id);
  const takeSnapshot = useTakeAccessSnapshot(id);
  const exportEvent = useExportAccessEvent();
  const [label, setLabel] = useState("");

  if (isLoading) return <p className="p-6">Loading…</p>;
  if (error || !data) return <p className="p-6">Event not found.</p>;

  const { event, present, entries, status_changes, snapshots } = data;
  const active = event.status === "running" || event.status === "paused";

  // Scans and status changes in one list, newest first.
  const log = [
    ...entries.map((e) => ({ at: e.recorded_at, entry: e, change: null })),
    ...status_changes.map((c) => ({
      at: c.changed_at,
      entry: null,
      change: c,
    })),
  ].sort((a, b) => b.at.localeCompare(a.at));

  const confirmDelete = (entry: AccessEntry) => {
    if (
      !confirm(
        `Delete the ${entry.direction === "in" ? "in" : "out"} entry of ${entry.member_name} at ${localTime(entry.recorded_at)}?`,
      )
    )
      return;
    deleteEntry.mutate(entry.id, {
      onError: (e) => toast.error(describeError(e)),
    });
  };

  return (
    <div className="max-w-4xl mx-auto p-4 sm:p-6 space-y-4">
      <Link
        to="/access-control"
        className="inline-flex items-center text-sm text-muted-foreground"
      >
        <ArrowLeft className="h-4 w-4 mr-1" /> All events
      </Link>

      <div className="flex flex-wrap items-center gap-3">
        <h1 className="text-2xl md:text-3xl font-bold tracking-tight">
          {event.name}
        </h1>
        <Badge variant={event.status === "running" ? "default" : "outline"}>
          {statusLabel[event.status]}
        </Badge>
      </div>
      <p className="text-muted-foreground">
        {localDate(event.event_date)} · Roles: {event.allowed_roles.join(", ")}
      </p>

      {/* Two columns on phones, with the scanner across the top. */}
      <div className="grid grid-cols-2 gap-2 sm:flex sm:flex-wrap">
        {active && (
          <Button asChild size="lg" className="col-span-2 h-14 text-base">
            <Link to={`/access-control/${event.id}/scan`}>
              <Camera className="h-5 w-5 mr-2" /> Open scanner
            </Link>
          </Button>
        )}
        {transitions[event.status].map(([next, text]) => (
          <Button
            key={next}
            variant={next === "ended" ? "destructive" : "outline"}
            size="lg"
            className="px-4"
            disabled={changeStatus.isPending}
            onClick={() => {
              if (
                next === "ended" &&
                !confirm(
                  `End "${event.name}"? Everyone still inside is checked out, and the event can't be reopened.`,
                )
              )
                return;
              changeStatus.mutate(
                { id: event.id, status: next },
                { onError: (e) => toast.error(describeError(e)) },
              );
            }}
          >
            {text}
          </Button>
        ))}
        <Button
          variant="outline"
          size="lg"
          className={cn(
            "px-4",
            // Alone on its row otherwise; fill it rather than leave a gap.
            transitions[event.status].length % 2 === 0 && "col-span-2",
          )}
          disabled={exportEvent.isPending}
          onClick={() =>
            exportEvent.mutate(event, {
              onError: (e) => toast.error(describeError(e)),
            })
          }
        >
          <Download className="h-5 w-5 mr-2" /> Download Excel
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Present now: {present.length}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-1 max-h-80 overflow-y-auto">
            {present.length === 0 && (
              <p className="text-sm text-muted-foreground">Nobody inside.</p>
            )}
            {present.map((m) => (
              <div key={m.user_id} className="text-sm min-w-0">
                <p className="truncate">{m.name}</p>
                <p className="truncate text-xs text-muted-foreground">
                  {m.email}
                </p>
              </div>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Snapshots</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Record who is inside right now, e.g. just before a vote.
            </p>
            <div className="flex gap-2">
              <Input
                className="text-base sm:text-sm"
                value={label}
                placeholder="Label (optional), e.g. Vote 1: chair"
                onChange={(e) => setLabel(e.target.value)}
              />
              <Button
                disabled={takeSnapshot.isPending}
                onClick={() =>
                  takeSnapshot.mutate(label.trim() || null, {
                    onSuccess: (s) => {
                      toast.success(
                        `Snapshot saved: ${s.present.length} present`,
                      );
                      setLabel("");
                    },
                    onError: (e) => toast.error(describeError(e)),
                  })
                }
              >
                Save
              </Button>
            </div>
            {snapshots.map((s) => (
              <div key={s.id} className="text-sm flex justify-between gap-2">
                <span>
                  {s.label ?? "Snapshot"}{" "}
                  <span className="text-muted-foreground">
                    {localTime(s.taken_at)}
                  </span>
                </span>
                <span className="font-semibold">
                  {s.present.length} present
                </span>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Log ({entries.length})</CardTitle>
        </CardHeader>
        <CardContent>
          {/* Phones: one compact row per entry instead of a wide table. */}
          <ul className="sm:hidden divide-y">
            {log.map((row, i) =>
              row.entry ? (
                <li key={row.entry.id} className="flex items-center gap-3 py-2">
                  <span className="w-12 shrink-0 text-sm tabular-nums text-muted-foreground">
                    {localTime(row.at)}
                  </span>
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-medium">
                      {row.entry.member_name}
                    </p>
                    <p className="truncate text-xs text-muted-foreground">
                      {methodLabel[row.entry.method]}
                      {row.entry.recorded_by_name &&
                        ` · ${row.entry.recorded_by_name}`}
                    </p>
                  </div>
                  <Badge
                    variant={
                      row.entry.direction === "in" ? "default" : "outline"
                    }
                  >
                    {row.entry.direction === "in" ? "In" : "Out"}
                  </Badge>
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label="Delete entry"
                    onClick={() => row.entry && confirmDelete(row.entry)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </li>
              ) : (
                <li
                  key={`status-${i}`}
                  className="flex items-center gap-3 py-2 -mx-2 px-2 bg-muted/40"
                >
                  <span className="w-12 shrink-0 text-sm tabular-nums text-muted-foreground">
                    {localTime(row.at)}
                  </span>
                  <p className="min-w-0 flex-1 truncate text-sm">
                    <span className="font-medium">
                      {row.change && statusLabel[row.change.status]}
                    </span>
                    {row.change?.changed_by_name && (
                      <span className="text-muted-foreground">
                        {` · ${row.change.changed_by_name}`}
                      </span>
                    )}
                  </p>
                </li>
              ),
            )}
          </ul>

          <div className="hidden sm:block overflow-x-auto">
            <table className="w-full text-sm">
              <thead className="text-left text-muted-foreground">
                <tr>
                  <th className="py-1 pr-3">Time</th>
                  <th className="py-1 pr-3">Name</th>
                  <th className="py-1 pr-3">Event</th>
                  <th className="py-1 pr-3">Method</th>
                  <th className="py-1 pr-3">By</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {log.map((row, i) =>
                  row.entry ? (
                    <tr key={row.entry.id} className="border-t">
                      <td className="py-1 pr-3">{localTime(row.at)}</td>
                      <td className="py-1 pr-3">{row.entry.member_name}</td>
                      <td className="py-1 pr-3">
                        {row.entry.direction === "in" ? "In" : "Out"}
                      </td>
                      <td className="py-1 pr-3">
                        {methodLabel[row.entry.method]}
                      </td>
                      <td className="py-1 pr-3">
                        {row.entry.recorded_by_name ?? ""}
                      </td>
                      <td className="py-1 text-right">
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-7 w-7"
                          aria-label="Delete entry"
                          onClick={() => row.entry && confirmDelete(row.entry)}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </td>
                    </tr>
                  ) : (
                    <tr key={`status-${i}`} className="border-t bg-muted/40">
                      <td className="py-1 pr-3">{localTime(row.at)}</td>
                      <td className="py-1 pr-3 font-medium" colSpan={3}>
                        {row.change ? statusLabel[row.change.status] : ""}
                      </td>
                      <td className="py-1 pr-3">
                        {row.change?.changed_by_name ?? ""}
                      </td>
                      <td />
                    </tr>
                  ),
                )}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
