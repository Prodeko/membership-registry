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
import { Input } from "@/components/ui/input";
import { downloadCsv } from "@/lib/utils";
import {
  useApplyRoleImport,
  usePreviewRoleImport,
  type RoleImportPreview,
  type RoleImportReport,
} from "@/lib/api";

export default function RoleImportPanel() {
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<RoleImportPreview | null>(null);
  const [report, setReport] = useState<RoleImportReport | null>(null);

  const previewMutation = usePreviewRoleImport();
  const applyMutation = useApplyRoleImport();

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
    applyMutation.mutate(file, {
      onSuccess: (r) => {
        setReport(r);
        if (r.fatal_error) toast.error(r.fatal_error);
        else
          toast.success(
            `Assigned: ${r.created} created, ${r.updated} updated, ${r.unchanged} unchanged, ${r.skipped + r.failed} problems`,
          );
      },
    });
  };

  const canApply =
    !!preview && !preview.fatal_error && !applyMutation.isPending;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Import role assignments</CardTitle>
        <CardDescription>
          Columns: email, role_name, valid_from (YYYY-MM-DD), valid_until
          (optional). The member and role must already exist. Re-importing the
          same email/role/valid_from overwrites valid_until.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <Input
          type="file"
          accept=".csv"
          onChange={(e) => onFile(e.target.files?.[0] ?? null)}
        />
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

        {preview && !report && <RolePreviewTable preview={preview} />}
        {report && <RoleReportTable report={report} />}
      </CardContent>
    </Card>
  );
}

function RolePreviewTable({ preview }: { preview: RoleImportPreview }) {
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
              <th className="px-3 py-2 font-medium">Role</th>
              <th className="px-3 py-2 font-medium">Action</th>
              <th className="px-3 py-2 font-medium">Changes</th>
            </tr>
          </thead>
          <tbody>
            {preview.rows.map((r) => (
              <tr key={r.line} className="border-t">
                <td className="px-3 py-2">{r.line}</td>
                <td className="px-3 py-2">{r.email}</td>
                <td className="px-3 py-2">{r.role_name}</td>
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

function roleReportToCsv(report: RoleImportReport): string {
  const header = "line,email,role_name,outcome,detail,changes";
  const esc = (s: string) => `"${s.replace(/"/g, '""')}"`;
  const lines = report.rows.map((r) =>
    [
      r.line,
      esc(r.email),
      esc(r.role_name),
      r.outcome,
      esc(r.detail ?? ""),
      esc(r.changes.join("; ")),
    ].join(","),
  );
  return [header, ...lines].join("\n");
}

function RoleReportTable({ report }: { report: RoleImportReport }) {
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
              roleReportToCsv(report),
              `role_import_report_${new Date().toISOString()}.csv`,
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
              <th className="px-3 py-2 font-medium">Role</th>
              <th className="px-3 py-2 font-medium">Outcome</th>
              <th className="px-3 py-2 font-medium">Detail</th>
            </tr>
          </thead>
          <tbody>
            {report.rows.map((r) => (
              <tr key={r.line} className="border-t">
                <td className="px-3 py-2">{r.line}</td>
                <td className="px-3 py-2">{r.email}</td>
                <td className="px-3 py-2">{r.role_name}</td>
                <td className="px-3 py-2">{r.outcome}</td>
                <td className="px-3 py-2 text-muted-foreground">
                  {r.detail ?? ""}
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
