import { useBackfillGoogleGroups, useGetGoogleGroups } from "@/lib/api";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Label } from "../ui/label";
import { Switch } from "../ui/switch";

function GoogleGroupsSyncCard() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [remove, setRemove] = useState(false);
  const { data: status, isLoading } = useGetGoogleGroups();
  const backfill = useBackfillGoogleGroups();

  const handleSync = () => {
    backfill.mutate(
      { remove },
      {
        onSuccess: (data) => {
          const parts = [`Checked ${data.users_processed} member(s)`];
          if (data.added > 0) parts.push(`${data.added} add(s)`);
          if (data.removed > 0) parts.push(`${data.removed} removal(s)`);
          if (data.failed > 0) parts.push(`${data.failed} failed`);
          const msg = parts.join(", ");
          if (data.failed > 0) toast.warning(msg);
          else toast.success(msg);
          setDialogOpen(false);
        },
        onError: () => {
          toast.error("Google Groups sync failed");
        },
      },
    );
  };

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Sync Google Groups</CardTitle>
          <CardDescription>
            Add every member who belongs on a mailing list to it. Group
            membership normally updates on its own when roles, language,
            attributes or email change; use this to catch up members who joined
            while no sync was running.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : !status?.configured ? (
            <p className="text-sm text-muted-foreground">
              Google Groups sync is not configured on this server.
            </p>
          ) : (
            <>
              <div className="text-sm space-y-1">
                <p className="font-medium">Groups</p>
                <ul className="list-disc pl-5">
                  {status.groups.map((group) => (
                    <li key={group}>{group}</li>
                  ))}
                </ul>
              </div>

              <div className="flex items-center gap-2">
                <Switch
                  id="google-groups-remove"
                  checked={remove}
                  onCheckedChange={setRemove}
                />
                <Label
                  htmlFor="google-groups-remove"
                  className="text-sm cursor-pointer"
                >
                  Also remove registry members who no longer qualify
                </Label>
              </div>

              <Button onClick={() => setDialogOpen(true)}>Sync groups</Button>
            </>
          )}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Confirm Google Groups sync</DialogTitle>
            <DialogDescription>
              {remove
                ? "Every member who qualifies for a group will be added to it, and registry members who don't will be removed. Addresses that aren't in the registry are never touched."
                : "Every member who qualifies for a group will be added to it. Nobody will be removed."}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={backfill.isPending}
            >
              Cancel
            </Button>
            <Button
              variant={remove ? "destructive" : "default"}
              onClick={handleSync}
              disabled={backfill.isPending}
            >
              {backfill.isPending ? "Syncing…" : "Sync"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default GoogleGroupsSyncCard;
