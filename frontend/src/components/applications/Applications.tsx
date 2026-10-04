import { QueryKey, useGetApplications, useGetTargetableRoles } from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { attributeColumn, columns } from "./columns";
import { APPLICATION_STATUSES } from "@/lib/constants";
import { Badge } from "../ui/badge";
import { useEffect, useMemo } from "react";
import { capitalizeFirstLetter } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";
import { Link, useSearchParams } from "react-router";
import { buttonVariants } from "../ui/button";

const Applications = () => {
  // The status filter lives in the URL so returning from an application
  // (or reloading) keeps the list the user was viewing.
  const [searchParams, setSearchParams] = useSearchParams();
  const statusParam = searchParams.get("status");
  const selectedStatus =
    APPLICATION_STATUSES.find((s) => s === statusParam) ?? "pending";
  const setSelectedStatus = (status: string) =>
    setSearchParams({ status }, { replace: true });

  const queryClient = useQueryClient();

  // One column per attribute asked on any targetable role's form, in the
  // order they first appear, slotted in before the actions column.
  const { data: targetableRoles } = useGetTargetableRoles();
  const tableColumns = useMemo(() => {
    const names = [
      ...new Set((targetableRoles ?? []).flatMap((r) => r.form_attributes)),
    ];
    const actionsIndex = columns.findIndex(
      (c) => "accessorKey" in c && c.accessorKey === "actions",
    );
    return [
      ...columns.slice(0, actionsIndex),
      ...names.map(attributeColumn),
      ...columns.slice(actionsIndex),
    ];
  }, [targetableRoles]);

  useEffect(() => {
    queryClient.invalidateQueries({ queryKey: [QueryKey.APPLICATIONS] });
  }, [queryClient, selectedStatus]);

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h1 className="text-4xl">Applications</h1>
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
            The set of roles and valid until dates that can be applied to in the
            application form.
          </TooltipContent>
        </Tooltip>
      </div>
      <div className="flex space-x-2">
        {APPLICATION_STATUSES.map((status) => (
          <Badge
            key={status}
            variant={selectedStatus === status ? "default" : "outline"}
            onClick={() => setSelectedStatus(status)}
          >
            {capitalizeFirstLetter(status)}
          </Badge>
        ))}
      </div>
      <DataTable
        columns={tableColumns}
        useFetchData={useGetApplications}
        initialColumnVisibility={{
          application_id: false,
          email: false,
          user_id: false,
        }}
        customFilters={{
          status: selectedStatus,
        }}
        searchColumn="full_name"
        modelName="applications"
      />
    </div>
  );
};

export default Applications;
