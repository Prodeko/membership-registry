import { useState } from "react";
import { RowSelectionState } from "@tanstack/react-table";
import { useGetRoleGroups } from "@/lib/api";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";
import CreateRoleGroupModal from "./CreateRoleGroupModal";
import RoleGroupBulkCommandDock from "./RoleGroupBulkCommandDock";
import PageHelp from "../ui/page-help";

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
      <PageHelp id="role-groups">
        <p>
          A <strong>role group</strong> is a named bundle of roles, for example
          everything a board member needs. Each group is mirrored to Keycloak as
          a group carrying those roles.
        </p>
        <ul>
          <li>
            Adding a member to a group gives them all of the group&apos;s roles
            in Keycloak, i.e. in apps that use Prodeko login, for the period you
            choose. It does not add the roles to their memberships in this
            registry.
          </li>
          <li>
            Changing a group&apos;s roles updates everyone in the group at once.
          </li>
          <li>
            Expired group memberships are removed automatically once a day.
          </li>
          <li>
            These are not the Google Workspace mailing-list groups; those follow
            members&apos; roles and are set up on the server.
          </li>
        </ul>
      </PageHelp>

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
