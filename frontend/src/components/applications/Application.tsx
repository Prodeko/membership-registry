import {
  QueryKey,
  useDeleteApplication,
  useGetApplication,
  useSetApplicationStatus,
} from "@/lib/api";
import { useNavigate, useParams, Link } from "react-router-dom";
import { Card } from "../ui/card";
import { Button } from "../ui/button";
import RoleBadge from "../ui/role-badge";
import { useQueryClient } from "@tanstack/react-query";
import { capitalizeFirstLetter } from "@/lib/utils";

const Application = () => {
  const { id = "" } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const {
    data: application,
    isLoading,
    error,
  } = useGetApplication(id, {
    enabled: id.length > 0,
  });

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

  const handleStatusUpdate = (status: "approved" | "rejected") => {
    updateStatus(
      { id: application.application_id, status },
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
        navigate("/applications");
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
            {new Date(application.timestamp).toLocaleDateString()}{" "}
            {new Date(application.timestamp).toLocaleTimeString()}
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

        <div className="flex gap-3 flex-wrap">
          {isPending && (
            <>
              <Button onClick={() => handleStatusUpdate("approved")}>
                Approve
              </Button>
              <Button
                variant="destructive"
                onClick={() => handleStatusUpdate("rejected")}
              >
                Reject
              </Button>
            </>
          )}
          <Button variant="destructive" onClick={handleDelete}>
            Delete
          </Button>
          <Button variant="outline" onClick={() => navigate("/applications")}>
            Back to applications
          </Button>
        </div>
      </Card>
    </div>
  );
};

export default Application;
