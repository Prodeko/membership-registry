import {
  QueryKey,
  useDeleteApplication,
  useGetApplication,
  useGetMemberAttributes,
  useGetTargetableRoles,
  useSetApplicationStatus,
} from "@/lib/api";
import { useNavigate, useParams, Link, useLocation } from "react-router";
import { Card } from "../ui/card";
import { Button } from "../ui/button";
import RoleBadge from "../ui/role-badge";
import { useQueryClient } from "@tanstack/react-query";
import { capitalizeFirstLetter } from "@/lib/utils";
import { MemberAttribute } from "@/common/generated/MemberAttribute";

const Application = () => {
  const { id = "" } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  // The filtered list we came from, if any (set by the applications table).
  const from = (location.state as { from?: string } | null)?.from;
  const backToList = from?.startsWith("/applications") ? from : "/applications";

  const {
    data: application,
    isLoading,
    error,
  } = useGetApplication(id, {
    enabled: id.length > 0,
  });

  const userId = application?.user_id ?? "";
  const { data: memberAttributes } = useGetMemberAttributes(userId, {
    enabled: userId.length > 0,
  });
  const { data: targetableRoles } = useGetTargetableRoles();

  const { mutate: updateStatus } = useSetApplicationStatus();
  const { mutate: deleteApplication } = useDeleteApplication();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (error) {
    return <div>Error: {error.message}</div>;
  }

  if (!application) {
    return <div>Application not found</div>;
  }

  const isPending =
    application.status === "pending" || application.status === "unpaid";

  // Attributes the applicant was asked for on this role's form come first,
  // in form order; any other attribute the member has a value for follows.
  // Values are the member's current ones, not a snapshot from submit time.
  const formAttributeNames =
    targetableRoles?.find(
      (r) =>
        r.role_name === application.role_name &&
        r.valid_until === application.valid_until,
    )?.form_attributes ?? [];
  const attributesByName = new Map(
    (memberAttributes ?? []).map((a) => [a.name, a]),
  );
  const formAttributes = formAttributeNames.map(
    (name) =>
      attributesByName.get(name) ??
      ({ name, value: null, required: false } as Pick<
        MemberAttribute,
        "name" | "value" | "required"
      >),
  );
  const otherAttributes = (memberAttributes ?? []).filter(
    (a) => a.value && !formAttributeNames.includes(a.name),
  );

  const handleStatusUpdate = (action: "approve" | "reject") => {
    updateStatus(
      { id: application.application_id, action },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({
            queryKey: [QueryKey.APPLICATIONS],
          });
        },
      },
    );
  };

  const handleDelete = () => {
    deleteApplication(application.application_id, {
      onSuccess: () => {
        queryClient.invalidateQueries({
          queryKey: [QueryKey.APPLICATIONS],
        });
        navigate(backToList);
      },
    });
  };

  return (
    <div className="flex justify-center align-middle p-14">
      <Card className="p-8 space-y-6 max-w-2xl w-full">
        <h1 className="text-3xl font-bold">Application</h1>

        <div className="grid grid-cols-2 gap-y-3 gap-x-6 text-sm">
          <span className="font-medium">Applicant</span>
          <Link
            to={`/members/${application.user_id}`}
            className="text-blue-600 hover:underline"
          >
            {application.full_name ?? "N/A"}
          </Link>

          <span className="font-medium">Email</span>
          <span>{application.email ?? "N/A"}</span>

          <span className="font-medium">Role</span>
          <span>
            <RoleBadge role={application.role_name} />
          </span>

          <span className="font-medium">Status</span>
          <span>
            {application.status
              ? capitalizeFirstLetter(application.status)
              : "N/A"}
          </span>

          <span className="font-medium">Valid until</span>
          <span>{new Date(application.valid_until).toLocaleDateString()}</span>

          <span className="font-medium">Created at</span>
          <span>
            {new Date(application.created_at).toLocaleDateString()}{" "}
            {new Date(application.created_at).toLocaleTimeString()}
          </span>

          {application.application_text && (
            <>
              <span className="font-medium">Application text</span>
              <span>{application.application_text}</span>
            </>
          )}

          {application.stripe_payment_id && (
            <>
              <span className="font-medium">Payment</span>
              <a
                href={`https://dashboard.stripe.com/payments/${application.stripe_payment_id}`}
                target="_blank"
                rel="noreferrer"
                className="text-blue-600 hover:underline"
              >
                View on Stripe
              </a>
            </>
          )}
        </div>

        {(formAttributes.length > 0 || otherAttributes.length > 0) && (
          <div className="space-y-3" data-testid="application-attributes">
            <h2 className="text-lg font-semibold">Attributes</h2>
            <div className="grid grid-cols-2 gap-y-2 gap-x-6 text-sm">
              {formAttributes.map((attr) => (
                <AttributeRow key={attr.name} attr={attr} />
              ))}
            </div>
            {otherAttributes.length > 0 && (
              <>
                {formAttributes.length > 0 && (
                  <p className="text-xs text-muted-foreground">
                    Other attributes on the member
                  </p>
                )}
                <div className="grid grid-cols-2 gap-y-2 gap-x-6 text-sm text-muted-foreground">
                  {otherAttributes.map((attr) => (
                    <AttributeRow key={attr.name} attr={attr} />
                  ))}
                </div>
              </>
            )}
          </div>
        )}

        <div className="flex gap-3 flex-wrap">
          {isPending && (
            <>
              <Button onClick={() => handleStatusUpdate("approve")}>
                Approve
              </Button>
              <Button
                variant="destructive"
                onClick={() => handleStatusUpdate("reject")}
              >
                Reject
              </Button>
            </>
          )}
          <Button variant="destructive" onClick={handleDelete}>
            Delete
          </Button>
          <Button variant="outline" onClick={() => navigate(backToList)}>
            Back to applications
          </Button>
        </div>
      </Card>
    </div>
  );
};

const AttributeRow = ({
  attr,
}: {
  attr: Pick<MemberAttribute, "name" | "value" | "required">;
}) => (
  <>
    <span className="font-mono">
      {attr.name}
      {attr.required && <span className="ml-0.5 text-destructive">*</span>}
    </span>
    <span data-testid={`application-attr-value-${attr.name}`}>
      {attr.value ? (
        attr.value
      ) : (
        <span className="text-muted-foreground italic">Not set</span>
      )}
    </span>
  </>
);

export default Application;
