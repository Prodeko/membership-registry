import {
  useExportAllMembers,
  useExportApplications,
  useExportAuditLogs,
  useExportRoles,
} from "@/lib/api";
import { UseMutationResult } from "@tanstack/react-query";
import { useState } from "react";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import SyncRolesCard from "./SyncRolesCard";

type Tab = "export" | "import" | "sync";

interface ExportConfig {
  title: string;
  description: string;
  useExport: () => UseMutationResult<void, Error, void>;
}

const exportConfigs: ExportConfig[] = [
  {
    title: "Members",
    description: "Export all members with their roles",
    useExport: useExportAllMembers,
  },
  {
    title: "Applications",
    description: "Export all membership applications",
    useExport: useExportApplications,
  },
  {
    title: "Audit Logs",
    description: "Export all audit log entries",
    useExport: useExportAuditLogs,
  },
  {
    title: "Roles",
    description: "Export all roles with member counts",
    useExport: useExportRoles,
  },
];

function ExportCard({ config }: { config: ExportConfig }) {
  const { mutate, isPending } = config.useExport();

  return (
    <Card>
      <CardHeader>
        <CardTitle>{config.title}</CardTitle>
        <CardDescription>{config.description}</CardDescription>
      </CardHeader>
      <CardContent>
        <Button onClick={() => mutate()} disabled={isPending}>
          {isPending ? "Exporting..." : "Export CSV"}
        </Button>
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
        <p className="text-muted-foreground">
          Import functionality coming soon.
        </p>
      )}
      {activeTab === "sync" && <SyncRolesCard />}
    </div>
  );
};

export default DataManagement;
