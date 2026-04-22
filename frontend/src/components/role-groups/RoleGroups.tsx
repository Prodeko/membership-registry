import { useState } from "react";
import { RowSelectionState } from "@tanstack/react-table";
import { useGetRoleGroups } from "@/lib/api";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";
import CreateRoleGroupModal from "./CreateRoleGroupModal";
import RoleGroupBulkCommandDock from "./RoleGroupBulkCommandDock";

export default function RoleGroups() {
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const { data: groups = [] } = useGetRoleGroups();

  const selectedGroupIds = Object.keys(rowSelection).filter(
    (k) => rowSelection[k],
  );

  const clearSelection = () => setRowSelection({});

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h1 className="text-4xl">Role Groups</h1>
        <CreateRoleGroupModal />
      </div>
      <DataTable
        columns={columns}
        useFetchData={useGetRoleGroups}
        searchColumn="name"
        modelName="groups"
        getRowId={(row) => row.id}
        rowSelection={rowSelection}
        onRowSelectionChange={setRowSelection}
      />
      <RoleGroupBulkCommandDock
        count={selectedGroupIds.length}
        selectedGroupIds={selectedGroupIds}
        groups={groups}
        onClear={clearSelection}
      />
    </div>
  );
}
