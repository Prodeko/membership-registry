import { useState } from "react";
import { toast } from "sonner";
import { useClearAutoDefaultValues, useGetAutoDefaultValues } from "@/lib/api";
import { describeError } from "@/lib/utils";
import { Badge } from "../ui/badge";
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

/**
 * Clears member-filled attribute values that registration filled in with the
 * attribute's default (before defaults were limited to admin-only
 * attributes), so members answer those attributes themselves.
 */
function AutoDefaultsCleanupCard() {
  const { data: rows, isLoading, error } = useGetAutoDefaultValues();
  const clear = useClearAutoDefaultValues();
  const [dialogOpen, setDialogOpen] = useState(false);

  const members = new Set((rows ?? []).map((r) => r.user_id)).size;
  const resubmitted = (rows ?? []).filter((r) => r.resubmitted).length;

  const handleClear = () =>
    clear.mutate(undefined, {
      onSuccess: (s) => {
        const msg = [
          `Cleared ${s.cleared} value(s)`,
          ...(s.keycloak_failed > 0
            ? [`${s.keycloak_failed} not cleared in Keycloak`]
            : []),
          ...(s.failed > 0 ? [`${s.failed} failed`] : []),
        ].join(", ");
        if (s.failed > 0 || s.keycloak_failed > 0) toast.warning(msg);
        else toast.success(msg);
        setDialogOpen(false);
      },
      onError: (e) => toast.error(`Clearing failed: ${describeError(e)}`),
    });

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Clear auto-filled attribute values</CardTitle>
          <CardDescription className="text-base">
            Registration used to fill every attribute&apos;s default value in
            for new members, including attributes members answer themselves
            (e.g. PoRa membership, study year). Those values looked like the
            member&apos;s own answers. This lists the ones still holding that
            default and clears them, so members fill them in themselves.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4 text-sm">
          {isLoading ? (
            <p className="text-muted-foreground">Checking…</p>
          ) : error ? (
            <p className="text-destructive">
              Couldn&apos;t load the list: {describeError(error)}
            </p>
          ) : !rows || rows.length === 0 ? (
            <p className="text-muted-foreground">
              Nothing to clear: no member holds an auto-filled default.
            </p>
          ) : (
            <>
              <p className="text-base">
                <strong>{rows.length}</strong> value(s) on{" "}
                <strong>{members}</strong> member(s).{" "}
                {resubmitted > 0 && (
                  <>
                    {resubmitted} of them were later sent again unchanged,
                    usually with the prefilled application form.
                  </>
                )}
              </p>
              <div className="max-h-80 overflow-auto rounded-md border">
                <table className="w-full">
                  <thead className="sticky top-0 bg-muted text-left">
                    <tr>
                      <th className="px-3 py-2 font-medium">Member</th>
                      <th className="px-3 py-2 font-medium">Attribute</th>
                      <th className="px-3 py-2 font-medium">Value</th>
                      <th className="px-3 py-2 font-medium">Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {rows.map((r) => (
                      <tr
                        key={`${r.user_id}:${r.attribute}`}
                        className="border-t"
                      >
                        <td className="px-3 py-2">
                          <div>{r.full_name || r.email}</div>
                          {r.full_name && (
                            <div className="text-muted-foreground">
                              {r.email}
                            </div>
                          )}
                        </td>
                        <td className="px-3 py-2 font-mono">{r.attribute}</td>
                        <td className="px-3 py-2">{r.values.join(", ")}</td>
                        <td className="px-3 py-2">
                          {r.resubmitted ? (
                            <Badge variant="outline">
                              sent again unchanged
                            </Badge>
                          ) : (
                            <Badge variant="secondary">untouched</Badge>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <Button variant="destructive" onClick={() => setDialogOpen(true)}>
                Clear {rows.length} value(s)
              </Button>
            </>
          )}
        </CardContent>
      </Card>
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Clear auto-filled values?</DialogTitle>
            <DialogDescription asChild>
              <div className="space-y-2">
                <p>
                  {rows?.length ?? 0} value(s) on {members} member(s) will be
                  cleared, in Keycloak too. Each clear is recorded in the audit
                  log; the values can&apos;t be restored automatically.
                </p>
                <p>
                  Members are asked to fill in required attributes on their next
                  login. If Google groups are in use, members whose PoRa answer
                  is cleared leave that group until they answer again.
                </p>
              </div>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={clear.isPending}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleClear}
              disabled={clear.isPending}
            >
              {clear.isPending ? "Clearing…" : "Clear"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default AutoDefaultsCleanupCard;
