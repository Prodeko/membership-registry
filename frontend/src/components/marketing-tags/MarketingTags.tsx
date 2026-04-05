import { useGetMarketingTags } from "@/lib/api";
import { DataTable } from "@/components/ui/data-table";
import { columns } from "./columns";
import MarketingTagFormModal from "./MarketingTagFormModal";

const MarketingTags = () => {
  return (
    <div className="space-y-4">
      <div className="flex justify-between">
        <h1 className="text-4xl">Marketing tags</h1>
        <MarketingTagFormModal />
      </div>

      <div className="rounded-md border bg-muted/40 p-4 text-sm text-muted-foreground">
        <p>
          This page configures which Mailchimp tags users can toggle on their
          profile mail preferences page. Users only see the tags listed here.
        </p>
        <p className="mt-2">
          If you want to manage tags that users <em>cannot</em> edit themselves
          (internal segments, one-off campaign tags), do that directly in the
          Mailchimp UI — do not add them here.
        </p>
        <p className="mt-2">
          The <code>label</code> field is the Mailchimp tag identifier. If the
          tag does not already exist in Mailchimp, Mailchimp will create it
          automatically the first time a user opts in. Deleting a row here only
          removes the tag from the user-editable list; existing Mailchimp
          subscribers keep it until you remove it in the Mailchimp UI.
        </p>
        <p className="mt-2">
          <strong>Auto-apply</strong> means new users are automatically opted
          into the tag on registration (and on the "resubscribe" button). It
          does <em>not</em> backfill existing subscribers — if you want to tag
          everyone currently on the list, do that as a bulk operation in the
          Mailchimp UI.
        </p>
      </div>

      <DataTable
        columns={columns}
        useFetchData={useGetMarketingTags}
        searchColumn="label"
        modelName="marketingTags"
      />
    </div>
  );
};

export default MarketingTags;
