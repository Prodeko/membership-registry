import { useCleanupExpiredRoles, useGetRolesStats } from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";
import CreateRoleModal from "./CreateRoleModal";
import { Link } from "react-router-dom";
import { Button, buttonVariants } from "../ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";

const Roles = () => {
  const queryClient = useQueryClient();
  const cleanupMutation = useCleanupExpiredRoles();

  const handleCleanupExpired = () => {
    cleanupMutation.mutate(undefined, {
      onSuccess: (data) => {
        toast.success(
          data.synced > 0
            ? `Removed ${data.synced} expired role(s) from Keycloak`
            : "No expired roles to clean up",
        );
        queryClient.invalidateQueries({ queryKey: ["roles"] });
      },
      onError: () => {
        toast.error("Failed to clean up expired roles");
      },
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between">
        <h1 className="text-4xl">Roles</h1>
        <div className="space-x-4">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="outline"
                  onClick={handleCleanupExpired}
                  disabled={cleanupMutation.isPending}
                >
                  {cleanupMutation.isPending
                    ? "Cleaning up..."
                    : "Clean up expired"}
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                Remove expired role memberships from Keycloak.
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
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
        modelName="roles"
      />
    </div>
  );
};

export default Roles;
