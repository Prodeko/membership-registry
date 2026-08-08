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
  useApplyAttributeImport,
  usePreviewAttributeImport,
  type AttributeImportPreview,
  type AttributeImportReport,
} from "@/lib/api";

export default function AttributeImportPanel() {
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<AttributeImportPreview | null>(null);
  const [report, setReport] = useState<AttributeImportReport | null>(null);

  const previewMutation = usePreviewAttributeImport();
  const applyMutation = useApplyAttributeImport();

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
            `Imported: ${r.created} created, ${r.updated} updated, ${r.unchanged} unchanged, ${r.skipped + r.failed} problems`,
          );
      },
    });
  };

  const canApply =
    !!preview && !preview.fatal_error && !applyMutation.isPending;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Import attribute definitions</CardTitle>
        <CardDescription>
          Columns: name (required), description, allowed_values (separated by{" "}
          <code>|</code>), default_value, editable_by (admin/user/both),
          sync_to_keycloak (true/false). Existing definitions (matched by name)
          are updated; empty cells keep the current value, <code>null</code>{" "}
          clears it. New definitions default to editable_by=admin,
          sync_to_keycloak=false.
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

        {preview && !report && <AttributePreviewTable preview={preview} />}
        {report && <AttributeReportTable report={report} />}
      </CardContent>
    </Card>
  );
}

function AttributePreviewTable({
  preview,
}: {
  preview: AttributeImportPreview;
}) {
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
              <th className="px-3 py-2 font-medium">Name</th>
              <th className="px-3 py-2 font-medium">Action</th>
              <th className="px-3 py-2 font-medium">Changes</th>
            </tr>
          </thead>
          <tbody>
            {preview.rows.map((r) => (
              <tr key={r.line} className="border-t">
                <td className="px-3 py-2">{r.line}</td>
                <td className="px-3 py-2">{r.name}</td>
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

function attributeReportToCsv(report: AttributeImportReport): string {
  const header = "line,name,outcome,detail,changes";
  const esc = (s: string) => `"${s.replace(/"/g, '""')}"`;
  const lines = report.rows.map((r) =>
    [
      r.line,
      esc(r.name),
      r.outcome,
      esc(r.detail ?? ""),
      esc(r.changes.join("; ")),
    ].join(","),
  );
  return [header, ...lines].join("\n");
}

function AttributeReportTable({ report }: { report: AttributeImportReport }) {
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
              attributeReportToCsv(report),
              `attribute_import_report_${new Date().toISOString()}.csv`,
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
              <th className="px-3 py-2 font-medium">Name</th>
              <th className="px-3 py-2 font-medium">Outcome</th>
              <th className="px-3 py-2 font-medium">Detail</th>
            </tr>
          </thead>
          <tbody>
            {report.rows.map((r) => (
              <tr key={r.line} className="border-t">
                <td className="px-3 py-2">{r.line}</td>
                <td className="px-3 py-2">{r.name}</td>
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
