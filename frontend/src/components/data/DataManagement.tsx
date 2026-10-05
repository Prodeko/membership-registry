import {
  useExportAllMembers,
  useExportApplications,
  useExportAuditLogs,
  useExportRoles,
} from "@/lib/api";
import { UseMutationResult } from "@tanstack/react-query";
import { ReactNode, useState } from "react";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import AttributeImportPanel from "./AttributeImportPanel";
import GoogleGroupsSyncCard from "./GoogleGroupsSyncCard";
import MailchimpResyncCard from "./MailchimpResyncCard";
import MemberImportPanel from "./MemberImportPanel";
import RoleImportPanel from "./RoleImportPanel";
import SyncRolesCard from "./SyncRolesCard";

type Tab = "export" | "import" | "sync";

interface ExportConfig {
  title: string;
  description: string;
  columns: string[];
  notes: ReactNode[];
  useExport: () => UseMutationResult<void, Error, void>;
}

const exportConfigs: ExportConfig[] = [
  {
    title: "Members",
    description: "Every member, one row per member.",
    columns: [
      "user_id",
      "first_name",
      "last_name",
      "full_name",
      "home_municipality",
      "email_notifications",
      "email",
      "role_names",
    ],
    notes: [
      <>
        <code>role_names</code>: every role the member has had, including ended
        ones, comma-separated.
      </>,
      <>
        Language and attribute values are not included, so this file can&apos;t
        be imported back as is.
      </>,
    ],
    useExport: useExportAllMembers,
  },
  {
    title: "Applications",
    description: "Every membership application, one row per application.",
    columns: [
      "application_id",
      "user_id",
      "full_name",
      "email",
      "role_name",
      "valid_until",
      "created_at",
      "status",
      "stripe_payment_id",
      "optional_roles",
      "application_text",
    ],
    notes: [
      <>
        <code>status</code>: Unpaid, Pending, Approved or Rejected.
      </>,
      <>
        <code>created_at</code> is an ISO 8601 timestamp in UTC (e.g.{" "}
        <code>2026-10-05T09:30:00+00:00</code>).
      </>,
    ],
    useExport: useExportApplications,
  },
  {
    title: "Audit Logs",
    description: "Every audit log entry: who changed what and when.",
    columns: [
      "id",
      "actor_user_id",
      "actor_name",
      "action",
      "entity_type",
      "entity_id",
      "details",
      "created_at",
    ],
    notes: [
      <>
        <code>details</code> holds the change as JSON. An empty actor means the
        system did it (e.g. a scheduled job).
      </>,
    ],
    useExport: useExportAuditLogs,
  },
  {
    title: "Roles",
    description: "Every role with its member counts, one row per role.",
    columns: [
      "name",
      "color",
      "description",
      "member_count",
      "active_member_count",
    ],
    notes: [
      <>
        <code>member_count</code>: everyone who has ever held the role.{" "}
        <code>active_member_count</code>: memberships valid today.
      </>,
    ],
    useExport: useExportRoles,
  },
];

function ExportCard({ config }: { config: ExportConfig }) {
  const { mutate, isPending } = config.useExport();

  return (
    <Card className="flex flex-col">
      <CardHeader>
        <CardTitle>{config.title}</CardTitle>
        <CardDescription className="text-base">
          {config.description}
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-4 text-base">
        <div className="space-y-2">
          <p className="text-sm font-medium">
            CSV file (comma-separated) with these columns:
          </p>
          <div className="flex flex-wrap gap-1.5">
            {config.columns.map((c) => (
              <code key={c} className="rounded bg-muted px-1.5 py-0.5 text-sm">
                {c}
              </code>
            ))}
          </div>
        </div>
        <ul className="list-disc space-y-1 pl-5">
          {config.notes.map((note, i) => (
            <li key={i}>{note}</li>
          ))}
        </ul>
        <div className="mt-auto">
          <Button onClick={() => mutate()} disabled={isPending}>
            {isPending ? "Exporting..." : "Export CSV"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

const DataManagement = () => {
  const [activeTab, setActiveTab] = useState<Tab>("export");

  return (
    <div className="space-y-6">
      <h1 className="text-4xl">Data Management</h1>
      <div className="flex space-x-2">
        <Button
          variant={activeTab === "export" ? "default" : "outline"}
          onClick={() => setActiveTab("export")}
        >
          Export
        </Button>
        <Button
          variant={activeTab === "import" ? "default" : "outline"}
          onClick={() => setActiveTab("import")}
        >
          Import
        </Button>
        <Button
          variant={activeTab === "sync" ? "default" : "outline"}
          onClick={() => setActiveTab("sync")}
        >
          Sync
        </Button>
      </div>
      {activeTab === "export" && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {exportConfigs.map((config) => (
            <ExportCard key={config.title} config={config} />
          ))}
        </div>
      )}
      {activeTab === "import" && (
        <div className="grid grid-cols-1 gap-4">
          <MemberImportPanel />
          <RoleImportPanel />
          <AttributeImportPanel />
        </div>
      )}
      {activeTab === "sync" && (
        <div className="grid grid-cols-1 gap-4">
          <SyncRolesCard />
          <GoogleGroupsSyncCard />
          <MailchimpResyncCard />
        </div>
      )}
    </div>
  );
};

export default DataManagement;
