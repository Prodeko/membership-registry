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

interface MissingRoleRow {
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

function SyncRolesCard() {
  const [dialogOpen, setDialogOpen] = useState(false);
  const { data: syncStatus, isLoading } = useGetKeycloakSyncStatus();
  const syncMutation = useSyncMissingKeycloakRoles();

  const { missingUserIds, totalMissing, totalExtras, totalUsersOutOfSync } =
    useMemo(() => {
      const statuses = syncStatus?.statuses ?? {};
      const ids: string[] = [];
      let missing = 0;
      let extras = 0;
      for (const [userId, s] of Object.entries(statuses)) {
        if (s.missing_in_keycloak.length > 0) ids.push(userId);
        missing += s.missing_in_keycloak.length;
        extras += s.extra_in_keycloak.length;
      }
      return {
        missingUserIds: ids,
        totalMissing: missing,
        totalExtras: extras,
        totalUsersOutOfSync: Object.keys(statuses).length,
      };
    }, [syncStatus]);

  const { data: members } = useGetMembersWithIds(missingUserIds);

  const missingRows: MissingRoleRow[] = useMemo(() => {
    const statuses = syncStatus?.statuses ?? {};
    const rows: MissingRoleRow[] = [];
    for (const userId of missingUserIds) {
      const s = statuses[userId];
      if (!s) continue;
      const name = displayNameFor(userId, members);
      for (const roleName of s.missing_in_keycloak) {
        rows.push({ userId, displayName: name, roleName });
      }
    }
    rows.sort(
      (a, b) =>
        a.displayName.localeCompare(b.displayName) ||
        a.roleName.localeCompare(b.roleName),
    );
    return rows;
  }, [syncStatus, missingUserIds, members]);

  const handleSync = () => {
    syncMutation.mutate(undefined, {
      onSuccess: (data) => {
        const msg =
          data.failed > 0
            ? `Added ${data.added} role(s) to Keycloak; ${data.failed} failed`
            : data.added > 0
              ? `Added ${data.added} role(s) to Keycloak`
              : "Everything was already in sync";
        if (data.failed > 0) toast.warning(msg);
        else toast.success(msg);
        setDialogOpen(false);
      },
      onError: () => {
        toast.error("Keycloak role sync failed");
      },
    });
  };

  const nothingToSync = totalMissing === 0;

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Sync roles to Keycloak</CardTitle>
          <CardDescription>
            Add role memberships that exist in the registry but are missing in
            Keycloak. Registry is the source of truth.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading drift…</p>
          ) : (
            <div className="text-sm space-y-1">
              <p>
                <span className="font-medium">{totalUsersOutOfSync}</span>{" "}
                user(s) out of sync
              </p>
              <p>
                <span className="font-medium">{totalMissing}</span> role
                assignment(s) will be added to Keycloak
              </p>
              {totalExtras > 0 && (
                <p className="text-muted-foreground">
                  {totalExtras} extra role(s) in Keycloak not in registry —{" "}
                  <em>not touched by this sync</em>
                </p>
              )}
            </div>
          )}
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
              The following role assignments will be added to Keycloak. This
              action is add-only — no roles will be removed from Keycloak.
            </DialogDescription>
          </DialogHeader>

          <div className="max-h-80 overflow-y-auto border rounded-md">
            <table className="w-full text-sm">
              <thead className="bg-muted text-left">
                <tr>
                  <th className="px-3 py-2 font-medium">Member</th>
                  <th className="px-3 py-2 font-medium">Role</th>
                </tr>
              </thead>
              <tbody>
                {missingRows.length === 0 ? (
                  <tr>
                    <td
                      className="px-3 py-4 text-center text-muted-foreground"
                      colSpan={2}
                    >
                      No changes to apply.
                    </td>
                  </tr>
                ) : (
                  missingRows.map((row, i) => (
                    <tr
                      key={`${row.userId}-${row.roleName}-${i}`}
                      className="border-t"
                    >
                      <td className="px-3 py-2">{row.displayName}</td>
                      <td className="px-3 py-2">{row.roleName}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
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
              disabled={syncMutation.isPending || missingRows.length === 0}
            >
              {syncMutation.isPending
                ? "Syncing…"
                : `Sync ${missingRows.length} change(s)`}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default SyncRolesCard;
