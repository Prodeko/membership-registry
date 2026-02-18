import { useGetEmailTemplates } from "@/lib/api";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";
import EmailTemplateFormModal from "./EmailTemplateFormModal";

const EmailTemplates = () => {
  return (
    <div className="space-y-4">
      <div className="flex justify-between">
        <h1 className="text-4xl">Email Templates</h1>
        <EmailTemplateFormModal />
      </div>
      <DataTable
        columns={columns}
        useFetchData={useGetEmailTemplates}
        searchColumn="name"
        modelName="emailTemplates"
      />
    </div>
  );
};

export default EmailTemplates;
