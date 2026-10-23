import { useGetTargetableRoles } from "@/lib/api";
import CreateTargetableRolesModal from "./CreateTargetableRolesModal";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";
import { Link } from "react-router";
import { buttonVariants } from "@/components/ui/button";

const TargetableRoles = () => {
  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h1 className="text-4xl">Targetable Roles</h1>
        <div className="flex gap-2">
          <Link
            to="/applications"
            className={buttonVariants({ variant: "outline" })}
          >
            Back to applications
          </Link>
          <CreateTargetableRolesModal />
        </div>
      </div>
      <DataTable
        columns={columns}
        useFetchData={useGetTargetableRoles}
        searchColumn="role_name"
        modelName="targetableRoles"
      />
    </div>
  );
};

export default TargetableRoles;
