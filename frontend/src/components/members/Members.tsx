import { useGetAllMembersWithRoles, useGetRoles } from "@/lib/api";
import { defaultFrom, defaultTo, stringsToOptions } from "@/lib/utils";
import React from "react";
import { DataTable } from "../ui/data-table";
import { DateRangePicker } from "../ui/date-range-picker";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import AddRolesModal from "./AddRolesModal";
import DeleteMembersModal from "./DeleteMembersModal";
import { columns } from "./columns";
import ExportMembersButton from "./ExportMembersButton";

const Members: React.FC = () => {

  const { data: roles, isLoading, error } = useGetRoles();
  const [selectedRoles, setSelectedRoles] = React.useState<Option[]>([]);
  const [selectedValidFrom, setSelectedValidFrom] = React.useState<
    Date
  >(defaultFrom);
  const [selectedValidUntil, setSelectedValidUntil] = React.useState<
    Date
  >(defaultTo);

  const onRoleChange = (selectedRoles: Option[]) => {
    setSelectedRoles(selectedRoles);
  };

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }


  return (
    <div className="space-y-4">
      <h1 className="text-4xl">Members</h1>
      <div className="flex space-x-2 flex-wrap">
        <MultipleSelector
          options={stringsToOptions(roles?.map((r) => r.name) ?? [])}
          onChange={onRoleChange}
          placeholder="Filter by role"
        />
        <DateRangePicker
          onUpdate={({ range }) => {
            setSelectedValidUntil(range.to ?? defaultTo);
            setSelectedValidFrom(range.from);
          }}
          initialDateFrom={defaultFrom}
          initialDateTo={defaultTo}
          locale="fi"
          showCompare={false}
          disabled={selectedRoles.length === 0}
        />
      </div>
      <DataTable
        modelName="members"
        columns={columns}
        useFetchData={useGetAllMembersWithRoles}
        searchColumn="first_name"
        initialColumnVisibility={{
          user_id: false,
          has_accepted_policies: false,
          home_municipality: false,
        }}
        customFilters={{
          roles: selectedRoles.map((role) => role.value),
          valid_until: selectedValidUntil,
          valid_from: selectedValidFrom,
        }}
        multipleRowActionElements={[
          (table, ids) => (
            <AddRolesModal
              userIds={ids}
              onClose={() => table.setRowSelection({})}
              disabled={ids.length === 0}
            />
          ),
          (table, ids) => (
            <DeleteMembersModal
              userIds={ids}
              onClose={() => table.setRowSelection({})}
              disabled={ids.length === 0}
            />
          ),
          (table) => (
            <ExportMembersButton
              table={table}
              customFilters={{
                roles: selectedRoles.map((role) => role.value),
                valid_until: selectedValidUntil,
                valid_from: selectedValidFrom,
              }}
            />
          ),
        ]}
        getRowId={(row) => row.user_id}
      />
    </div>
  );
};

export default Members;
