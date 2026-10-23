import { useGetTargetableRoles } from "@/lib/api";
import CreateTargetableRolesModal from "./CreateTargetableRolesModal";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";
import { Link, useLocation } from "react-router";
import { buttonVariants } from "@/components/ui/button";

const TargetableRoles = () => {
  // The page is linked from both Roles and Applications; go back to
  // whichever one the user came from (defaulting to Applications).
  const location = useLocation();
  const from = (location.state as { from?: string } | null)?.from;
  const fromRoles = from?.startsWith("/roles") ?? false;
  const backTo =
    from && (fromRoles || from.startsWith("/applications"))
      ? from
      : "/applications";
  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h1 className="text-4xl">Targetable Roles</h1>
        <div className="flex gap-2">
          <Link to={backTo} className={buttonVariants({ variant: "outline" })}>
            {fromRoles ? "Back to roles" : "Back to applications"}
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
