import { useGetTargetableRoles } from "@/lib/api";
import CreateTargetableRolesModal from "../CreateTargetableRolesModal";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";

const TargetableRoles = () => {
  const { data: targetableRoles, isLoading } = useGetTargetableRoles();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex justify-between">
        <h1 className="text-4xl">Targetable Roles</h1>
        <CreateTargetableRolesModal />
      </div>
      <DataTable columns={columns} useFetchData={useGetTargetableRoles} searchColumn="role_name"/>
    </div>
  );
};

export default TargetableRoles;
