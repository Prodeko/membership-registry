import { useGetMailchimp, useResyncMailchimp } from "@/lib/api";
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
import { Input } from "../ui/input";
import { Label } from "../ui/label";

/** "2026, 2027" → [2026, 2027]; null if any entry isn't a 4-digit year. */
function parseYears(text: string): number[] | null {
  const parts = text
    .split(/[\s,]+/)
    .map((p) => p.trim())
    .filter(Boolean);
  if (parts.length === 0 || !parts.every((p) => /^\d{4}$/.test(p))) {
    return null;
  }
  return [...new Set(parts.map(Number))];
}

function MailchimpResyncCard() {
  const thisYear = new Date().getFullYear();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [yearsText, setYearsText] = useState(`${thisYear}, ${thisYear + 1}`);
  const { data: status, isLoading } = useGetMailchimp();
  const resync = useResyncMailchimp();
  const years = parseYears(yearsText);

  const handleResync = () => {
    if (!years) return;
    resync.mutate(
      { years },
      {
        onSuccess: (data) => {
          const parts = [
            `Checked ${data.contacts_checked} contact(s)`,
            `${data.members_qualifying} qualifying member(s)`,
            `${data.archived} archived`,
            `${data.added} added`,
            `${data.restored} restored`,
          ];
          if (data.skipped_unsubscribed > 0)
            parts.push(`${data.skipped_unsubscribed} unsubscribed, skipped`);
          if (data.failed > 0) parts.push(`${data.failed} failed`);
          const msg = parts.join(", ");
          if (data.failed > 0) toast.warning(msg);
          else toast.success(msg);
          setDialogOpen(false);
        },
        onError: () => {
          toast.error(
            "Mailchimp resync failed. Nothing is changed if no member qualifies for the given years.",
          );
        },
      },
    );
  };

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Resync Mailchimp</CardTitle>
          <CardDescription>
            Rebuild the Mailchimp audience from the registry. Every contact
            without a membership in one of the given years is archived first,
            including contacts that aren't in the registry. Then every member
            with such a membership is added, or restored if archived. Members
            who unsubscribed themselves are never re-added.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : !status?.configured ? (
            <p className="text-sm text-muted-foreground">
              Mailchimp sync is not configured on this server.
            </p>
          ) : (
            <>
              <div className="space-y-1 max-w-xs">
                <Label htmlFor="mailchimp-years">Membership years</Label>
                <Input
                  id="mailchimp-years"
                  value={yearsText}
                  onChange={(e) => setYearsText(e.target.value)}
                  placeholder="2026, 2027"
                />
                {!years && (
                  <p className="text-sm text-destructive">
                    Enter one or more years, e.g. 2026, 2027
                  </p>
                )}
              </div>

              <Button
                variant="destructive"
                onClick={() => setDialogOpen(true)}
                disabled={!years}
              >
                Resync Mailchimp
              </Button>
            </>
          )}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Confirm Mailchimp resync</DialogTitle>
            <DialogDescription>
              Every subscribed or pending contact without a membership in{" "}
              {years?.join(" or ")} will be archived. Members with such a
              membership will be added. Archived contacts can be restored in
              Mailchimp; nothing is deleted permanently.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={resync.isPending}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleResync}
              disabled={resync.isPending || !years}
            >
              {resync.isPending ? "Resyncing…" : "Resync"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default MailchimpResyncCard;
