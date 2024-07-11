import { useGetApplications } from "@/lib/api";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";

const Applications = () => {

  return (
    <div>
      <h1 className="text-4xl">Applications</h1>
      <DataTable
        columns={columns}
        useFetchData={useGetApplications}
        initialColumnVisibility={
          {
            application_id: false
          }
        }
      />
    </div>
  );
}

export default Applications;