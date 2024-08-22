import { useGetRolesStats } from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";
import CreateRoleModal from "./CreateRoleModal";
import { Link } from "react-router-dom";
import { buttonVariants } from "../ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "../ui/tooltip";

const Roles = () => {
  return (
    <div className="space-y-4">
      <div className="flex justify-between">
        <h1 className="text-4xl">Roles</h1>
        <div className="space-x-4">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Link
                  to={"/applications/targetable-roles"}
                  className={buttonVariants({ variant: "outline" })}
                >
                  Application targetable roles
                </Link>
              </TooltipTrigger>
              <TooltipContent>
                The set of roles and valid until dates that can be applied to in
                the application form.
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          <CreateRoleModal />
        </div>
      </div>
      <DataTable
        columns={columns}
        useFetchData={useGetRolesStats}
        searchColumn="name"
      />
    </div>
  );
};

export default Roles;
