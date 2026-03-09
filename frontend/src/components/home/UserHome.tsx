import {
  useGetMeMember,
  useGetTargetableRoles,
  useGetUserApplications,
} from "@/lib/api";
import { kebabCaseToTitleCase } from "@/lib/utils";
import { Clock, FileText } from "lucide-react";
import { Link } from "react-router-dom";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import { Separator } from "../ui/separator";

const statusConfig = {
  approved: { variant: "default" as const, label: "Approved" },
  rejected: { variant: "destructive" as const, label: "Rejected" },
  pending: { variant: "secondary" as const, label: "Pending" },
  unpaid: { variant: "outline" as const, label: "Unpaid" },
};

const UserHome = () => {
  const { data: member, isLoading: isMemberLoading } = useGetMeMember();
  const { data: applications, isLoading: isAppsLoading } =
    useGetUserApplications();
  const { data: targetableRoles } = useGetTargetableRoles();

  if (isMemberLoading || isAppsLoading) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">Loading...</p>
      </main>
    );
  }

  if (!member) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">Could not load your profile.</p>
      </main>
    );
  }

  return (
    <main className="flex justify-center min-h-screen w-screen px-4 py-20">
      <div className="max-w-2xl w-full space-y-6">
        <div>
          <h1 className="text-3xl font-bold">Welcome, {member.first_name}</h1>
          <p className="text-muted-foreground mt-1">
            Here's an overview of your membership status.
          </p>
        </div>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <FileText className="h-5 w-5" />
              Your applications
            </CardTitle>
            <CardDescription>
              Track the status of your membership applications.
            </CardDescription>
          </CardHeader>
          <CardContent>
            {!applications || applications.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                You haven't submitted any applications yet.
              </p>
            ) : (
              <div className="space-y-3">
                {applications.map((app) => {
                  const config = statusConfig[app.status];
                  const role = targetableRoles?.find(
                    (r) =>
                      r.role_name === app.role_name &&
                      r.valid_until === app.valid_until,
                  );
                  return (
                    <div key={app.application_id}>
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="font-medium">
                            {kebabCaseToTitleCase(app.role_name)}
                          </p>
                          <p className="text-sm text-muted-foreground flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            Valid until{" "}
                            {new Date(app.valid_until).toLocaleDateString()}
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          {role?.payment_link && app.status === "unpaid" && (
                            <a
                              href={role.payment_link}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="text-sm text-blue-500 hover:underline"
                            >
                              Pay fee
                            </a>
                          )}
                          <Badge variant={config.variant}>{config.label}</Badge>
                        </div>
                      </div>
                      <Separator className="mt-3" />
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Profile</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
              <span className="text-muted-foreground">Name</span>
              <span>{member.full_name}</span>
              <span className="text-muted-foreground">Email</span>
              <span>{member.email}</span>
              <span className="text-muted-foreground">Municipality</span>
              <span>{member.home_municipality}</span>
            </div>
          </CardContent>
        </Card>

        <Button asChild className="w-full">
          <Link to="/apply">Apply for membership</Link>
        </Button>
      </div>
    </main>
  );
};

export default UserHome;
