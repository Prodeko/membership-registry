import { useState } from "react";
import { Link } from "react-router";
import { ArrowLeft, Pencil, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { AccessEvent } from "@/common/types";
import {
  useDeleteAccessEvent,
  useGetAccessEvents,
  useGetAccessPass,
  useGetRoles,
  useSaveAccessEvent,
} from "@/lib/api";
import { describeError } from "@/lib/utils";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Card, CardContent } from "../ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import MultipleSelector from "../ui/multiple-selector";
import { localDate, statusLabel } from "./labels";

const today = () => new Date().toISOString().slice(0, 10);

function EventFormDialog({
  event,
  onClose,
}: {
  /** The event to edit, or null to create one. */
  event: AccessEvent | null;
  onClose: () => void;
}) {
  const { data: roles } = useGetRoles();
  const save = useSaveAccessEvent();
  const [name, setName] = useState(event?.name ?? "");
  const [date, setDate] = useState(event?.event_date ?? today());
  const [allowed, setAllowed] = useState<string[]>(event?.allowed_roles ?? []);

  const options = (roles ?? []).map((r) => ({ label: r.name, value: r.name }));
  const canSave = name.trim().length > 0 && allowed.length > 0 && !!date;

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{event ? "Edit event" : "New event"}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1">
            <Label htmlFor="access-event-name">Name</Label>
            <Input
              id="access-event-name"
              value={name}
              placeholder="e.g. Syysvaalikokous 2026"
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="access-event-date">Date</Label>
            <Input
              id="access-event-date"
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
          </div>
          <div className="space-y-1">
            <Label>Roles that get members in</Label>
            <MultipleSelector
              options={options}
              value={allowed.map((r) => ({ label: r, value: r }))}
              onChange={(opts) => setAllowed(opts.map((o) => o.value))}
              placeholder="Pick roles"
            />
            <p className="text-xs text-muted-foreground">
              A member gets in while holding any of these roles on the day they
              are scanned.
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button
            disabled={!canSave || save.isPending}
            onClick={() =>
              save.mutate(
                {
                  id: event?.id,
                  data: { name, event_date: date, allowed_roles: allowed },
                },
                {
                  onSuccess: () => {
                    toast.success(event ? "Event saved" : "Event created");
                    onClose();
                  },
                  onError: (e) => toast.error(describeError(e)),
                },
              )
            }
          >
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/// Lists meeting access control events. Door staff open them; admins also
/// create, edit and delete them.
export default function AccessEvents() {
  const { data: events, isLoading } = useGetAccessEvents();
  const { data: pass } = useGetAccessPass();
  const isAdmin = !!pass?.is_admin;
  const deleteEvent = useDeleteAccessEvent();
  const [editing, setEditing] = useState<AccessEvent | null | undefined>(
    undefined,
  );

  return (
    <div className="max-w-3xl mx-auto p-4 sm:p-6 space-y-4">
      <Link
        to={isAdmin ? "/members" : "/home"}
        className="inline-flex items-center text-sm text-muted-foreground"
      >
        <ArrowLeft className="h-4 w-4 mr-1" /> Back
      </Link>
      <div className="flex items-center justify-between gap-2">
        <h1 className="text-3xl font-bold tracking-tight">Access control</h1>
        {isAdmin && (
          <Button onClick={() => setEditing(null)}>
            <Plus className="h-4 w-4 mr-1" /> New event
          </Button>
        )}
      </div>

      {isLoading && <p>Loading…</p>}
      {events?.length === 0 && (
        <p className="text-muted-foreground">
          No events yet.
          {isAdmin && " Create one for the next meeting."}
        </p>
      )}

      {events?.map((event) => (
        <Card key={event.id}>
          <CardContent className="pt-6 flex items-center justify-between gap-3">
            <Link to={`/access-control/${event.id}`} className="flex-1 min-w-0">
              <p className="font-semibold truncate">{event.name}</p>
              <p className="text-sm text-muted-foreground">
                {localDate(event.event_date)} · {event.allowed_roles.join(", ")}
              </p>
            </Link>
            <Badge variant={event.status === "running" ? "default" : "outline"}>
              {statusLabel[event.status]}
            </Badge>
            {isAdmin && (
              <>
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={`Edit ${event.name}`}
                  onClick={() => setEditing(event)}
                >
                  <Pencil className="h-4 w-4" />
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={`Delete ${event.name}`}
                  onClick={() => {
                    if (
                      !confirm(
                        `Delete "${event.name}" and its whole attendance log? This can't be undone.`,
                      )
                    )
                      return;
                    deleteEvent.mutate(event.id, {
                      onSuccess: () => toast.success("Event deleted"),
                      onError: (e) => toast.error(describeError(e)),
                    });
                  }}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </>
            )}
          </CardContent>
        </Card>
      ))}

      {editing !== undefined && (
        <EventFormDialog
          event={editing}
          onClose={() => setEditing(undefined)}
        />
      )}
    </div>
  );
}
