import {
  MutationCache,
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import { AxiosError } from "axios";
import { toast } from "sonner";
import ApplicationForm from "./components/application-form/ApplicationForm";
import Application from "./components/applications/Application";
import Applications from "./components/applications/Applications";
import TargetableRoles from "./components/applications/targetable-roles/TargetableRoles";
import Callback from "./components/auth/Callback";
import ErrorPage from "./components/Error";
import Layout from "./components/layout/Layout";
import Member from "./components/members/Member";
import Members from "./components/members/Members";
import Role from "./components/roles/Role";
import Roles from "./components/roles/Roles";
import AuditLogs from "./components/audit-logs/AuditLogs";
import DataManagement from "./components/data/DataManagement";
import EmailTemplates from "./components/email-templates/EmailTemplates";
import MarketingTags from "./components/marketing-tags/MarketingTags";
import { ThemeProvider } from "./components/theme-provider";
import Success from "./components/application-form/Success";
import PaymentSuccess from "./components/payment/PaymentSuccess";
import UserHome from "./components/home/UserHome";
import ProfileEdit from "./components/profile/ProfileEdit";
import { TooltipProvider } from "./components/ui/tooltip";
import { Toaster } from "./components/ui/sonner";

function getErrorMessage(error: unknown): string {
  if (error instanceof AxiosError) {
    if (error.response?.data && typeof error.response.data === "string") {
      return error.response.data;
    }
    if (error.message) return error.message;
  }
  if (error instanceof Error) return error.message;
  return "An unexpected error occurred";
}

const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error) => {
      if (error instanceof AxiosError) {
        const status = error.response?.status;
        if (!status || status >= 500) {
          toast.error(getErrorMessage(error));
        }
      } else {
        toast.error(getErrorMessage(error));
      }
    },
  }),
  mutationCache: new MutationCache({
    onError: (error) => {
      toast.error(getErrorMessage(error));
    },
  }),
});

const router = createBrowserRouter([
  {
    path: "/error/:status",
    element: <ErrorPage />,
  },
  {
    path: "/",
    element: (
      <Layout>
        <Members />
      </Layout>
    ),
    errorElement: <ErrorPage />,
  },
  {
    path: "/members",
    element: (
      <Layout>
        <Members />
      </Layout>
    ),
  },
  {
    path: "/members/:id",
    element: (
      <Layout>
        <Member />
      </Layout>
    ),
  },
  {
    path: "/roles",
    element: (
      <Layout>
        <Roles />
      </Layout>
    ),
  },
  {
    path: "/roles/:id",
    element: (
      <Layout>
        <Role />
      </Layout>
    ),
  },
  {
    path: "/applications",
    element: (
      <Layout>
        <Applications />
      </Layout>
    ),
  },
  {
    path: "/applications/:id",
    element: (
      <Layout>
        <Application />
      </Layout>
    ),
  },
  {
    path: "/applications/targetable-roles",
    element: (
      <Layout>
        <TargetableRoles />
      </Layout>
    ),
  },
  {
    path: "/logs",
    element: (
      <Layout>
        <AuditLogs />
      </Layout>
    ),
  },
  {
    path: "/email-templates",
    element: (
      <Layout>
        <EmailTemplates />
      </Layout>
    ),
  },
  {
    path: "/marketing-tags",
    element: (
      <Layout>
        <MarketingTags />
      </Layout>
    ),
  },
  {
    path: "/data",
    element: (
      <Layout>
        <DataManagement />
      </Layout>
    ),
  },
  {
    path: "/home",
    element: <UserHome />,
  },
  {
    path: "/profile/edit",
    element: <ProfileEdit />,
  },
  {
    path: "/apply",
    element: <ApplicationForm />,
  },
  {
    path: "/apply/success",
    element: <Success />,
  },
  {
    path: "/payment/success",
    element: <PaymentSuccess />,
  },
  {
    path: "/auth/callback",
    element: <Callback />,
  },
]);

function App() {
  return (
    <ThemeProvider>
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <RouterProvider router={router} />
        </TooltipProvider>
      </QueryClientProvider>
      <Toaster />
    </ThemeProvider>
  );
}

export default App;
