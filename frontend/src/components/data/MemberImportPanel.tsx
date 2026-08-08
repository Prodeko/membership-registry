import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { downloadCsv } from "@/lib/utils";
import {
  useApplyMemberImport,
  usePreviewMemberImport,
  type MemberImportPreview,
  type MemberImportReport,
} from "@/lib/api";

export default function MemberImportPanel() {
  const [file, setFile] = useState<File | null>(null);
  const [sendInvites, setSendInvites] = useState(false);
  const [preview, setPreview] = useState<MemberImportPreview | null>(null);
  const [report, setReport] = useState<MemberImportReport | null>(null);

  const previewMutation = usePreviewMemberImport();
  const applyMutation = useApplyMemberImport();

  const onFile = (f: File | null) => {
    setFile(f);
    setPreview(null);
    setReport(null);
  };

  const runPreview = () => {
    if (!file) return;
    previewMutation.mutate(file, {
      onSuccess: (p) => {
        setReport(null);
        setPreview(p);
        if (p.fatal_error) toast.error(p.fatal_error);
      },
    });
  };

  const runApply = () => {
    if (!file) return;
    applyMutation.mutate(
      { file, sendInvites },
      {
        onSuccess: (r) => {
          setReport(r);
          if (r.fatal_error) toast.error(r.fatal_error);
          else
            toast.success(
              `Imported: ${r.created} created, ${r.updated} updated, ${r.unchanged} unchanged, ${r.skipped + r.failed} problems`,
            );
        },
      },
    );
  };

  const canApply =
    !!preview && !preview.fatal_error && !applyMutation.isPending;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Import members</CardTitle>
        <CardDescription>
          Columns: email (required), first_name, last_name, home_municipality,
          language, email_notifications, plus any attribute key as its own
          column. Existing members (matched by email) are updated; unknown
          emails create a Keycloak account. Set an attribute cell to{" "}
          <code>null</code> to clear it. language (fi/en, default fi) also picks
          the invite email language.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Input
          type="file"
          accept=".csv"
          onChange={(e) => onFile(e.target.files?.[0] ?? null)}
        />

        <div className="flex items-center gap-2">
          <Checkbox
            id="send-invites"
            checked={sendInvites}
            onCheckedChange={(v) => setSendInvites(!!v)}
          />
          <Label htmlFor="send-invites">
            Send set-password invite to new accounts
          </Label>
        </div>

        <div className="flex gap-2">
          <Button
            onClick={runPreview}
            disabled={!file || previewMutation.isPending}
          >
            {previewMutation.isPending ? "Checking..." : "Preview"}
          </Button>
          <Button onClick={runApply} disabled={!canApply}>
            {applyMutation.isPending ? "Importing..." : "Confirm import"}
          </Button>
        </div>

        {preview && !report && <MemberPreviewTable preview={preview} />}
        {report && <MemberReportTable report={report} />}
      </CardContent>
    </Card>
  );
}

function MemberPreviewTable({ preview }: { preview: MemberImportPreview }) {
  if (preview.fatal_error) {
    return <p className="text-destructive">{preview.fatal_error}</p>;
  }
  return (
    <div className="space-y-2">
      <p className="text-sm text-muted-foreground">
        {preview.create_count} create · {preview.update_count} update ·{" "}
        {preview.unchanged_count} unchanged · {preview.error_count} error
      </p>
      <div className="border rounded-md max-h-96 overflow-y-auto">
        <table className="w-full text-sm">
          <thead className="bg-muted text-left">
            <tr>
              <th className="px-3 py-2 font-medium">#</th>
              <th className="px-3 py-2 font-medium">Email</th>
              <th className="px-3 py-2 font-medium">Action</th>
              <th className="px-3 py-2 font-medium">Changes</th>
            </tr>
          </thead>
          <tbody>
            {preview.rows.map((r) => (
              <tr key={r.line} className="border-t">
                <td className="px-3 py-2">{r.line}</td>
                <td className="px-3 py-2">{r.email}</td>
                <td className="px-3 py-2">
                  {r.error ? (
                    <span className="text-destructive">{r.error}</span>
                  ) : (
                    r.action
                  )}
                </td>
                <td className="px-3 py-2 text-muted-foreground">
                  {r.changes.map((c) => (
                    <div key={c}>{c}</div>
                  ))}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function reportToCsv(report: MemberImportReport): string {
  const header = "line,email,outcome,detail,warning,changes";
  const esc = (s: string) => `"${s.replace(/"/g, '""')}"`;
  const lines = report.rows.map((r) =>
    [
      r.line,
      esc(r.email),
      r.outcome,
      esc(r.detail ?? ""),
      esc(r.warning ?? ""),
      esc(r.changes.join("; ")),
    ].join(","),
  );
  return [header, ...lines].join("\n");
}

function MemberReportTable({ report }: { report: MemberImportReport }) {
  if (report.fatal_error) {
    return <p className="text-destructive">{report.fatal_error}</p>;
  }
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          {report.created} created · {report.updated} updated ·{" "}
          {report.unchanged} unchanged · {report.skipped} skipped ·{" "}
          {report.failed} failed
        </p>
        <Button
          variant="outline"
          size="sm"
          onClick={() =>
            downloadCsv(
              reportToCsv(report),
              `member_import_report_${new Date().toISOString()}.csv`,
            )
          }
        >
          Download report
        </Button>
      </div>
      <div className="border rounded-md max-h-96 overflow-y-auto">
        <table className="w-full text-sm">
          <thead className="bg-muted text-left">
            <tr>
              <th className="px-3 py-2 font-medium">#</th>
              <th className="px-3 py-2 font-medium">Email</th>
              <th className="px-3 py-2 font-medium">Outcome</th>
              <th className="px-3 py-2 font-medium">Detail</th>
            </tr>
          </thead>
          <tbody>
            {report.rows.map((r) => (
              <tr key={r.line} className="border-t">
                <td className="px-3 py-2">{r.line}</td>
                <td className="px-3 py-2">{r.email}</td>
                <td className="px-3 py-2">{r.outcome}</td>
                <td className="px-3 py-2 text-muted-foreground">
                  {r.detail ?? r.warning ?? ""}
                  {r.changes.map((c) => (
                    <div key={c}>{c}</div>
                  ))}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
