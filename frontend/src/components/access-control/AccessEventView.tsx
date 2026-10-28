import { useState } from "react";
import { Link, useParams } from "react-router";
import { ArrowLeft, Camera, Download, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { AccessEventStatus } from "@/common/types";
import {
  useChangeAccessEventStatus,
  useDeleteAccessEntry,
  useExportAccessEvent,
  useGetAccessEvent,
  useTakeAccessSnapshot,
} from "@/lib/api";
import { describeError } from "@/lib/utils";
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

      <div className="flex flex-wrap gap-2">
        {active && (
          <Button asChild size="lg">
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
              <p key={m.user_id} className="text-sm">
                {m.name}{" "}
                <span className="text-muted-foreground">{m.email}</span>
              </p>
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
          <div className="overflow-x-auto">
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
                {[
                  ...entries.map((e) => ({ at: e.recorded_at, entry: e })),
                  ...status_changes.map((c) => ({
                    at: c.changed_at,
                    change: c,
                  })),
                ]
                  .sort((a, b) => b.at.localeCompare(a.at))
                  .map((row, i) =>
                    "entry" in row && row.entry ? (
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
                            onClick={() => {
                              const entry = row.entry;
                              if (
                                !confirm(
                                  `Delete the ${entry.direction === "in" ? "in" : "out"} entry of ${entry.member_name} at ${localTime(entry.recorded_at)}?`,
                                )
                              )
                                return;
                              deleteEntry.mutate(entry.id, {
                                onError: (e) => toast.error(describeError(e)),
                              });
                            }}
                          >
                            <Trash2 className="h-3.5 w-3.5" />
                          </Button>
                        </td>
                      </tr>
                    ) : (
                      <tr key={`status-${i}`} className="border-t bg-muted/40">
                        <td className="py-1 pr-3">{localTime(row.at)}</td>
                        <td className="py-1 pr-3 font-medium" colSpan={3}>
                          {"change" in row && row.change
                            ? statusLabel[row.change.status]
                            : ""}
                        </td>
                        <td className="py-1 pr-3">
                          {"change" in row && row.change
                            ? (row.change.changed_by_name ?? "")
                            : ""}
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
