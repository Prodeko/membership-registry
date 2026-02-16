import { useGetAuditLogs } from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";

const AuditLogs: React.FC = () => {
  return (
    <div className="space-y-4">
      <h1 className="text-4xl">Audit Logs</h1>
      <DataTable
        modelName="audit_logs"
        columns={columns}
        useFetchData={useGetAuditLogs}
        searchColumn="action"
        initialColumnVisibility={{}}
        getRowId={(row) => row.id}
      />
    </div>
  );
};

export default AuditLogs;
