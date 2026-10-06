import { useGetEmailTemplates } from "@/lib/api";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";
import EmailTemplateFormModal from "./EmailTemplateFormModal";
import PageHelp from "../ui/page-help";

const EmailTemplates = () => {
  return (
    <div className="space-y-4">
      <div className="flex justify-between">
        <h1 className="text-4xl">Email Templates</h1>
        <EmailTemplateFormModal />
      </div>
      <PageHelp id="email-templates">
        <p>
          Email templates are the texts of the emails the registry sends. Create
          a template here, then pick it where it is used:
        </p>
        <ul>
          <li>
            <strong>Application approved / rejected</strong>: per role, under{" "}
            <strong>Application targetable roles</strong> (button on the
            Applications page).
          </li>
          <li>
            <strong>Renewal reminders</strong>: on a role&apos;s page, under
            Renewal settings, together with the reminder days (e.g. 30, 7 and 1
            days before expiry).
          </li>
        </ul>
        <p>
          Each template has a Finnish and an English version; members get the
          one in their language, and Finnish when there is no English version.
          The body is HTML.
        </p>
        <p>
          <strong>Placeholders</strong> in curly braces are filled in when the
          email is sent. <code>{"{name}"}</code> and{" "}
          <code>{"{role_name}"}</code> work in every email;{" "}
          <code>{"{payment_link}"}</code>, <code>{"{valid_until}"}</code> and{" "}
          <code>{"{expires_in}"}</code> only in renewal reminders. The editor
          shows where a template is used and warns if a placeholder would not be
          filled there. Unknown placeholders are rejected when saving.
        </p>
      </PageHelp>

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
