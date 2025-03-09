import { QueryKey, useGetApplications } from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";
import { STATUSES } from "@/lib/constants";
import { Badge } from "../ui/badge";
import { useEffect, useState } from "react";
import { capitalizeFirstLetter } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";

const Applications = () => {
  const [selectedStatus, setSelectedStatus] = useState<string | null>('approved');

  const queryClient = useQueryClient();

  useEffect(() => {
    queryClient.invalidateQueries({ queryKey: [QueryKey.APPLICATIONS] });
  }, [queryClient, selectedStatus]);

  return (
    <div className="space-y-4">
      <h1 className="text-4xl">Applications</h1>
      <div className="flex space-x-2">
        {STATUSES.map((status) => (
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
