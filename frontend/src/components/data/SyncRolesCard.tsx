import {
  useGetKeycloakSyncStatus,
  useGetMembersWithIds,
  useSyncMissingKeycloakRoles,
} from "@/lib/api";
import { MemberWithRoles } from "@/common/types";
import { useMemo, useState } from "react";
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
import { Switch } from "../ui/switch";
import { Label } from "../ui/label";

interface RoleRow {
  userId: string;
  displayName: string;
  roleName: string;
}

function displayNameFor(userId: string, members: MemberWithRoles[] = []) {
  const m = members.find((mm) => mm.user_id === userId);
  if (!m) return userId;
  const name = [m.first_name, m.last_name].filter(Boolean).join(" ").trim();
  return name.length > 0 ? `${name} (${m.email})` : m.email;
}

function buildRows(
  userIds: string[],
  getRoles: (userId: string) => string[],
  members: MemberWithRoles[] | undefined,
): RoleRow[] {
  const rows: RoleRow[] = [];
  for (const userId of userIds) {
    const name = displayNameFor(userId, members);
    for (const roleName of getRoles(userId)) {
      rows.push({ userId, displayName: name, roleName });
    }
  }
  rows.sort(
    (a, b) =>
      a.displayName.localeCompare(b.displayName) ||
      a.roleName.localeCompare(b.roleName),
  );
  return rows;
}

function RoleTable({ rows, emptyText }: { rows: RoleRow[]; emptyText: string }) {
  return (
    <div className="border rounded-md overflow-hidden">
      <table className="w-full text-sm">
        <thead className="bg-muted text-left">
          <tr>
            <th className="px-3 py-2 font-medium">Member</th>
            <th className="px-3 py-2 font-medium">Role</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td className="px-3 py-4 text-center text-muted-foreground" colSpan={2}>
                {emptyText}
              </td>
            </tr>
          ) : (
            rows.map((row, i) => (
              <tr key={`${row.userId}-${row.roleName}-${i}`} className="border-t">
                <td className="px-3 py-2">{row.displayName}</td>
                <td className="px-3 py-2">{row.roleName}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

function SyncRolesCard() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [removeExpired, setRemoveExpired] = useState(false);
  const { data: syncStatus, isLoading } = useGetKeycloakSyncStatus();
  const syncMutation = useSyncMissingKeycloakRoles();

  const { missingUserIds, expiredUserIds, totals } = useMemo(() => {
    const statuses = syncStatus?.statuses ?? {};
    const missingIds: string[] = [];
    const expiredIds: string[] = [];
    let totalMissing = 0;
    let totalExpired = 0;
    let totalUnmanaged = 0;
    for (const [userId, s] of Object.entries(statuses)) {
      if (s.missing_in_keycloak.length > 0) missingIds.push(userId);
      if (s.expired_in_keycloak.length > 0) expiredIds.push(userId);
      totalMissing += s.missing_in_keycloak.length;
      totalExpired += s.expired_in_keycloak.length;
      totalUnmanaged += s.unmanaged_in_keycloak.length;
    }
    return {
      missingUserIds: missingIds,
      expiredUserIds: expiredIds,
      totals: {
        missing: totalMissing,
        expired: totalExpired,
        unmanaged: totalUnmanaged,
        usersOutOfSync: Object.keys(statuses).length,
      },
    };
  }, [syncStatus]);

  const allRelevantIds = useMemo(
    () => [...new Set([...missingUserIds, ...expiredUserIds])],
    [missingUserIds, expiredUserIds],
  );
  const { data: members } = useGetMembersWithIds(allRelevantIds);

  const missingRows = useMemo(() => {
    const statuses = syncStatus?.statuses ?? {};
    return buildRows(
      missingUserIds,
      (id) => statuses[id]?.missing_in_keycloak ?? [],
      members,
    );
  }, [syncStatus, missingUserIds, members]);

  const expiredRows = useMemo(() => {
    const statuses = syncStatus?.statuses ?? {};
    return buildRows(
      expiredUserIds,
      (id) => statuses[id]?.expired_in_keycloak ?? [],
      members,
    );
  }, [syncStatus, expiredUserIds, members]);

  const handleSync = () => {
    syncMutation.mutate(
      { removeExpired },
      {
        onSuccess: (data) => {
          const parts: string[] = [];
          if (data.added > 0) parts.push(`Added ${data.added} role(s)`);
          if (data.removed > 0) parts.push(`Removed ${data.removed} role(s)`);
          const hasFailures = data.failed > 0 || data.remove_failed > 0;
          if (data.failed > 0) parts.push(`${data.failed} add(s) failed`);
          if (data.remove_failed > 0) parts.push(`${data.remove_failed} removal(s) failed`);
          const msg = parts.length > 0 ? parts.join(", ") : "Everything was already in sync";
          if (hasFailures) toast.warning(msg);
          else toast.success(msg);
          setDialogOpen(false);
        },
        onError: () => {
          toast.error("Keycloak role sync failed");
        },
      },
    );
  };

  const nothingToSync = totals.missing === 0 && (!removeExpired || totals.expired === 0);

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Sync roles to Keycloak</CardTitle>
          <CardDescription>
            Add active role memberships that are missing in Keycloak. Registry is the source of truth.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading drift…</p>
          ) : (
            <div className="text-sm space-y-1">
              <p>
                <span className="font-medium">{totals.usersOutOfSync}</span>{" "}
                user(s) out of sync
              </p>
              <p>
                <span className="font-medium">{totals.missing}</span> active role
                assignment(s) will be added to Keycloak
              </p>
              {totals.expired > 0 && (
                <p className={removeExpired ? "text-destructive" : "text-muted-foreground"}>
                  <span className="font-medium">{totals.expired}</span> expired role(s) in
                  Keycloak —{" "}
                  {removeExpired ? "will be removed" : "removable (enable option below)"}
                </p>
              )}
              {totals.unmanaged > 0 && (
                <p className="text-muted-foreground">
                  {totals.unmanaged} unmanaged role(s) in Keycloak (no registry record) —{" "}
                  <em>never touched by sync</em>
                </p>
              )}
            </div>
          )}

          <div className="flex items-center gap-2">
            <Switch
              id="remove-expired"
              checked={removeExpired}
              onCheckedChange={setRemoveExpired}
            />
            <Label htmlFor="remove-expired" className="text-sm cursor-pointer">
              Also remove expired registry roles from Keycloak
            </Label>
          </div>

          <Button
            onClick={() => setDialogOpen(true)}
            disabled={isLoading || nothingToSync}
          >
            Preview &amp; sync
          </Button>
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Confirm Keycloak sync</DialogTitle>
            <DialogDescription>
              {removeExpired
                ? "The following role assignments will be added or removed in Keycloak."
                : "The following active role assignments will be added to Keycloak. No roles will be removed."}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 max-h-96 overflow-y-auto">
            {missingRows.length > 0 && (
              <div>
                <p className="text-sm font-medium mb-1">
                  Will be added ({missingRows.length})
                </p>
                <RoleTable rows={missingRows} emptyText="No roles to add." />
              </div>
            )}
            {removeExpired && (
              <div>
                <p className="text-sm font-medium mb-1 text-destructive">
                  Will be removed ({expiredRows.length})
                </p>
                <RoleTable rows={expiredRows} emptyText="No expired roles to remove." />
              </div>
            )}
            {missingRows.length === 0 && (!removeExpired || expiredRows.length === 0) && (
              <p className="text-sm text-center text-muted-foreground py-4">
                No changes to apply.
              </p>
            )}
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={syncMutation.isPending}
            >
              Cancel
            </Button>
            <Button
              onClick={handleSync}
              disabled={
                syncMutation.isPending ||
                (missingRows.length === 0 && expiredRows.length === 0)
              }
            >
              {syncMutation.isPending
                ? "Syncing…"
                : `Sync ${missingRows.length + (removeExpired ? expiredRows.length : 0)} change(s)`}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default SyncRolesCard;
