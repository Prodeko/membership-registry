import { useGetAllMembers, useGetRoles } from "@/lib/api";
import React from "react";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import { capitalizeFirstLetter } from "@/lib/utils";

const Members: React.FC = () => {
  const { data, isLoading, error } = useGetRoles();
  const [selectedRoles, setSelectedRoles] = React.useState<Option[]>([]);

  const onRoleChange = (selectedRoles: Option[]) => {
    setSelectedRoles(selectedRoles);
  }

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  const options: Option[] = data?.map((role) => ({
    label: capitalizeFirstLetter(role.name),
    value: role.name,
  })) ?? []; 

  return (
    <div>
      <h1>Members</h1>
      <MultipleSelector options={options} onChange={onRoleChange}/>
      <DataTable
        columns={columns}
        useFetchData={useGetAllMembers}
        initialColumnVisibility={{
          user_id: false,
          has_accepted_policies: false,
        }}
        customFilters={{
          roles: selectedRoles.map((role) => role.value)
        }}
      />
    </div>
  );
};

export default Members;
