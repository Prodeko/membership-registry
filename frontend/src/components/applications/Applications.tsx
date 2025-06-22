import { QueryKey, useGetApplications } from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";
import { APPLICATION_STATUSES } from "@/lib/constants";
import { Badge } from "../ui/badge";
import { useEffect, useState } from "react";
import { capitalizeFirstLetter } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";
import { Link } from "react-router-dom";
import { buttonVariants } from "../ui/button";

const Applications = () => {
  const [selectedStatus, setSelectedStatus] = useState<string | null>(
    "pending"
  );

  const queryClient = useQueryClient();

  useEffect(() => {
    queryClient.invalidateQueries({ queryKey: [QueryKey.APPLICATIONS] });
  }, [queryClient, selectedStatus]);

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h1 className="text-4xl">Applications</h1>
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
        columns={columns}
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
