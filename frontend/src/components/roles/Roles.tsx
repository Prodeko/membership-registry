import {
  useCleanupExpiredRoles,
  useGetRolesCount,
  useGetRolesStats,
} from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";
import CreateRoleModal from "./CreateRoleModal";
import { Link } from "react-router";
import { Button, buttonVariants } from "../ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import PageHelp from "../ui/page-help";

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
      <PageHelp id="roles">
        <p>
          A <strong>role</strong> is a membership or position a member can hold,
          such as a membership type or the board. A member holds a role for a
          period (valid from / valid until); click a member&apos;s row on the
          member list to add or end roles.
        </p>
        <ul>
          <li>
            Roles are mirrored to Keycloak, so apps that use Prodeko login see
            who holds which role. Creating a role here also creates it in
            Keycloak.
          </li>
          <li>
            <strong>Member count</strong> is everyone who has ever held the
            role; <strong>active</strong> counts memberships valid today.
          </li>
          <li>
            Expired memberships are removed from Keycloak automatically once a
            day. <strong>Clean up expired</strong> does it right away.
          </li>
          <li>
            Open a role to set up <strong>renewal</strong>: a Stripe payment
            link, how long a renewal lasts, when members can renew, reminder
            emails and the banner on the member home page.
          </li>
          <li>
            Which roles people can <strong>apply</strong> for is set under{" "}
            <strong>Application targetable roles</strong> (button above). Those
            roles also decide who is on the Mailchimp list.
          </li>
        </ul>
      </PageHelp>

      <DataTable
        columns={columns}
        useFetchData={useGetRolesStats}
        useCount={useGetRolesCount}
        searchColumn="name"
        modelName="roles"
      />
    </div>
  );
};

export default Roles;
